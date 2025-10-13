// Copyright 2023-2025 The Android Open Source Project

use crate::chip_error::ChipError;
use crate::client_method;
use crate::packet_streamer::PacketStreamerApi;
use netsim_proto::configuration::Controller as RootcanalController;
use netsim_proto::model::chip::BleBeacon;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::NetsimRadioStats as ProtoRadioStats;
use std::fmt;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

// CHIP SERVICE
//
// This module implements the actor model for managing simulated chips.
//
// The "service" is the actor's message-processing loop, which would be
// implemented in a separate task that owns the `mpsc::Receiver<ChipRequest>`.
//
// The "client" (`ChipClient`) is a handle to the actor that allows other parts
// of the system to send messages to it.

/// A unique identifier for a simulated chip, represented as a u32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipIdentifier(pub u32);

impl ChipIdentifier {
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// The error type for the `ChipClient`.
#[derive(Error, Debug)]
pub enum ClientError {
    /// An error occurred when sending a command to the service.
    #[error("Failed to send command to service: {0}")]
    Send(String),

    /// An error occurred when receiving a response from the service.
    #[error("Service did not respond: {0}")]
    Response(#[from] tokio::sync::oneshot::error::RecvError),

    /// An operation-specific error occurred from the chip service.
    #[error("Operation failed: {0}")]
    Chip(#[from] ChipError),
}

/// A reply channel for sending the result of an operation back to the caller.
pub type Responder<T> = oneshot::Sender<Result<T, ChipError>>;

/// Defines the message protocol for the chip service actor.
///
/// Each variant corresponds to a specific command that can be sent to the
/// service. For commands that require a response, a `Responder` channel is
/// included.
#[derive(Debug)]
pub enum ChipRequest {
    /// Create a new chip.
    CreateChip {
        /// The parameters for the new chip.
        params: CreateChipParams,
        /// The channel to send the result.
        respond_to: Responder<()>,
    },
    /// Update an existing chip.
    UpdateChip {
        /// The ID of the chip to patch.
        id: ChipIdentifier,
        /// The patch to apply to the chip.
        chip: ProtoChip,
        /// The channel to send the updated chip state back on.
        respond_to: Responder<ProtoChip>,
    },
    /// Get the state of a chip.
    GetChip {
        /// The ID of the chip to retrieve.
        id: ChipIdentifier,
        /// The channel to send the chip's state back on.
        respond_to: Responder<ProtoChip>,
    },
    /// Delete a chip.
    DeleteChip {
        /// The ID of the chip to delete.
        id: ChipIdentifier,
        /// The channel to send the operation result back on.
        respond_to: Responder<()>,
    },
    /// Reset all event counters for a chip.
    ResetChip {
        /// The ID of the chip to reset.
        id: ChipIdentifier,
    },
    /// Get radio statistics for all chips.
    GetChipStatistics {
        /// The channel to send the statistics back on.
        respond_to: Responder<Vec<ProtoRadioStats>>,
    },
    /// Get the total number of chips for testing purposes.
    GetChipCountForTesting {
        /// The channel to send the count back on.
        respond_to: Responder<usize>,
    },
    /// Command the service to shut down gracefully.
    Shutdown,
}

/// The parameters for the `CreateChip` command.
pub struct CreateChipParams {
    /// The ID of the chip to create.
    pub id: ChipIdentifier,

    /// The parameters for the chip to create.
    pub address: String,
    pub bt_properties: Option<RootcanalController>,
    pub ble_beacon: Option<BleBeacon>,

    /// The packet streamer to use for the chip.
    pub packet_streamer: Box<dyn PacketStreamerApi>,
}

impl fmt::Debug for CreateChipParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateChipParams")
            .field("id", &self.id)
            .field("address", &self.address)
            .field("bt_properties", &self.bt_properties)
            .field("ble_beacon", &self.ble_beacon)
            .field("packet_streamer", &"Box<dyn PacketStreamerApi>")
            .finish()
    }
}

impl fmt::Display for ChipIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// Chip Actor CLIENT
// =============================================================================

/// A client handle for interacting with the chip service actor.
///
/// This client provides a high-level API for sending `ChipRequest` messages to
/// the service over an `mpsc` channel. It abstracts away the channel and
/// `oneshot` responder boilerplate for each command.
#[derive(Clone)]
pub struct ChipClient {
    sender: mpsc::Sender<ChipRequest>,
}

impl ChipClient {
    /// Creates a new `ChipClient` handle.
    ///
    /// This function connects the client to the service's message channel.
    ///
    /// # Arguments
    ///
    /// * `sender` - The `mpsc` sender half of the channel for sending `ChipRequest`s.
    pub fn new(sender: mpsc::Sender<ChipRequest>) -> Self {
        Self { sender }
    }

    /// Sends a shutdown command to the chip service.
    ///
    /// This is a fire-and-forget command; it does not wait for a response.
    pub async fn shutdown(&self) -> Result<(), ClientError> {
        self.sender
            .send(ChipRequest::Shutdown)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        Ok(())
    }
}

// Generate client methods.
client_method!(ChipClient => fn get_chip(id: ChipIdentifier) -> ProtoChip as ChipRequest::GetChip);
client_method!(ChipClient => fn update_chip(id: ChipIdentifier, chip: ProtoChip) -> ProtoChip as ChipRequest::UpdateChip);
client_method!(ChipClient => fn create_chip(params: CreateChipParams) -> () as ChipRequest::CreateChip);
client_method!(ChipClient => fn delete_chip(id: ChipIdentifier) -> () as ChipRequest::DeleteChip);
client_method!(ChipClient => fn get_chip_statistics() -> Vec<ProtoRadioStats> as ChipRequest::GetChipStatistics);
client_method!(ChipClient => fn get_chip_count_for_testing() -> usize as ChipRequest::GetChipCountForTesting);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet_streamer::MockPacketStreamerApi;

    #[tokio::test]
    async fn test_get_chip() {
        let (tx, mut rx) = mpsc::channel(1);
        let client = ChipClient::new(tx);

        // Spawn a task to handle the client call
        tokio::spawn(async move {
            let chip_id = ChipIdentifier(1);
            let _ = client.get_chip(chip_id).await;
        });

        // Receive the message and assert
        let received = rx.recv().await.unwrap();
        match received {
            ChipRequest::GetChip { id, respond_to: _ } => {
                assert_eq!(id, ChipIdentifier(1));
            }
            _ => panic!("Received incorrect ChipRequest variant"),
        }
    }

    #[tokio::test]
    async fn test_create_chip() {
        let (tx, mut rx) = mpsc::channel(1);
        let client = ChipClient::new(tx);
        let (mock_streamer, _, _) = MockPacketStreamerApi::new();

        let params = CreateChipParams {
            id: ChipIdentifier(2),
            address: "00:11:22:33:44:55".to_string(),
            bt_properties: None,
            ble_beacon: None,
            packet_streamer: Box::new(mock_streamer),
        };

        tokio::spawn(async move {
            let _ = client.create_chip(params).await;
        });

        let received = rx.recv().await.unwrap();
        match received {
            ChipRequest::CreateChip { params, respond_to: _ } => {
                assert_eq!(params.id, ChipIdentifier(2));
                assert_eq!(params.address, "00:11:22:33:44:55");
            }
            _ => panic!("Received incorrect ChipRequest variant"),
        }
    }

    #[tokio::test]
    async fn test_get_chip_statistics() {
        let (tx, mut rx) = mpsc::channel(1);
        let client = ChipClient::new(tx);

        tokio::spawn(async move {
            let _ = client.get_chip_statistics().await;
        });

        let received = rx.recv().await.unwrap();
        match received {
            ChipRequest::GetChipStatistics { respond_to: _ } => {
                // Correct variant received, nothing else to check
            }
            _ => panic!("Received incorrect ChipRequest variant"),
        }
    }
}
