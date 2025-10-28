//! This module provides the Cellular `Server` component.

use log::{info, warn};
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{ChipClient, ChipRequest};
use tokio::sync::mpsc;

/// Manages the state and interactions for simulated cellular chips.
///
/// This server follows an actor model, processing requests received via an mpsc channel.
#[derive(Clone, Default)]
pub struct Server {
    // TODO: Add cellular modem state fields
}

impl Server {
    /// Creates a new `Server` instance and a `ChipClient` to communicate with it.
    ///
    /// Returns a tuple of `(Server, ChipClient)`.
    pub fn new() -> (Self, ChipClient) {
        let (tx, _rx) = mpsc::channel(100); // Increased channel size
        (Server::default(), ChipClient::new(tx))
    }

    /// Runs the main event loop for the cellular server.
    ///
    /// This function listens for incoming `ChipRequest` messages and handles them accordingly.
    /// It will run until a `ChipRequest::Shutdown` message is received.
    pub async fn run(mut self, mut command_rx: mpsc::Receiver<ChipRequest>) {
        info!("CellServer started");
        while let Some(request) = command_rx.recv().await {
            match request {
                ChipRequest::Create { params, respond_to } => {
                    // TODO: Implement Create Chip
                    warn!("Create not implemented, params: {params:?}");
                    let _ = respond_to.send(Err(ChipError::Unsupported));
                }
                ChipRequest::Read { id, respond_to } => {
                    // TODO: Implement Read Chip State (GetState)
                    warn!("Read not implemented for chip {id}");
                    let _ = respond_to.send(Err(ChipError::ChipNotFound(id)));
                }
                ChipRequest::Update { id, chip, respond_to } => {
                    // TODO: Implement Update Chip State (SetState)
                    warn!("Update not implemented for chip {id}, chip: {chip:?}");
                    let _ = respond_to.send(Err(ChipError::ChipNotFound(id)));
                }
                ChipRequest::Delete { id, respond_to } => {
                    // TODO: Implement Delete Chip
                    warn!("Delete not implemented for chip {id}");
                    let _ = respond_to.send(Err(ChipError::ChipNotFound(id)));
                }
                ChipRequest::Reset { id } => {
                    info!("Resetting CellServer for chip {id}");
                    // TODO: Handle multi-chip reset if necessary, for now reset all
                    self.reset().await;
                }
                ChipRequest::GetStatistics { respond_to } => {
                    // TODO: Implement GetStatistics
                    warn!("GetStatistics not implemented");
                    let _ = respond_to.send(Err(ChipError::Unsupported));
                }
                ChipRequest::GetCountForTesting { respond_to } => {
                    let _ = respond_to.send(Ok(0)); // Placeholder
                }
                ChipRequest::Shutdown => {
                    info!("Shutting down CellServer");
                    break;
                }
            }
        }
        info!("CellServer stopped");
    }

    /// Resets the server state to default.
    pub async fn reset(&mut self) {
        *self = Self::default();
    }
}
