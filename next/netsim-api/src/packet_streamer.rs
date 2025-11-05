// Copyright 2023-2025 The Android Open Source Project

use async_trait::async_trait;
use thiserror::Error;

/// The error type for the netsim API.
#[derive(Error, Debug)]
pub enum PsError {
    /// An error occurred during I/O.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred during packet processing.
    #[error("Packet processing error: {0}")]
    Packet(String),
    /// The operation is not supported.
    #[error("Unsupported operation")]
    Unsupported,
}

/// A transport-agnostic abstraction for bidirectional packet I/O.
///
/// This trait is used by the simulation to communicate with the client (e.g.,
/// an Android Virtual Device) without being tied to a specific transport like
/// gRPC or file descriptors.
#[async_trait]
pub trait PacketStreamerApi: Send + Sync {
    /// Asynchronously reads the next packet from the client.
    /// Returns `Ok(None)` if the stream is closed gracefully.
    async fn read_packet(&mut self) -> Result<Option<Vec<u8>>, PsError>;

    /// Asynchronously writes a packet to the client.
    async fn write_packet(&mut self, packet: Vec<u8>) -> Result<(), PsError>;
}

#[cfg(test)]
use tokio::sync::mpsc as mock_mpsc;

#[cfg(test)]
#[derive(Debug)]
pub struct MockPacketStreamerApi {
    pub tx: mock_mpsc::UnboundedSender<Vec<u8>>,
    pub rx: mock_mpsc::UnboundedReceiver<Vec<u8>>,
}

#[cfg(test)]
impl MockPacketStreamerApi {
    pub fn new(
    ) -> (Self, mock_mpsc::UnboundedSender<Vec<u8>>, mock_mpsc::UnboundedReceiver<Vec<u8>>) {
        let (to_mock_tx, to_mock_rx) = mock_mpsc::unbounded_channel();
        let (from_mock_tx, from_mock_rx) = mock_mpsc::unbounded_channel();
        (Self { tx: from_mock_tx, rx: to_mock_rx }, to_mock_tx, from_mock_rx)
    }
}

#[cfg(test)]
#[async_trait]
impl PacketStreamerApi for MockPacketStreamerApi {
    async fn read_packet(&mut self) -> Result<Option<Vec<u8>>, PsError> {
        Ok(self.rx.recv().await)
    }

    async fn write_packet(&mut self, packet: Vec<u8>) -> Result<(), PsError> {
        self.tx.send(packet).map_err(|e| PsError::Packet(e.to_string()))
    }
}
