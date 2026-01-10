// Copyright 2024-2025 The Android Open Source Project
pub mod error;
pub mod fake_modem_network;
pub mod server;

pub use server::CellRunner;
pub use server::CellServer as Server;

use netsim_model::chip::RadioChipClient;
use tokio::sync::mpsc;

/// Creates a new Cell server runner and its client.
pub fn new() -> (CellRunner, RadioChipClient) {
    let (command_tx, command_rx) = mpsc::channel(10);
    (CellRunner::new(command_rx), RadioChipClient::new(command_tx))
}
