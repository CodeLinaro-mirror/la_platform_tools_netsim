// Rust definitions for statistics related structures
use serde::{Deserialize, Serialize};

/// Represents the statistics for a radio.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimRadioStats {
    /// The unique ID of the chip.
    pub id: u32,
    /// The name of the chip.
    pub name: String,
    /// Number of bytes transmitted.
    pub tx_bytes: u64,
    /// Number of bytes received.
    pub rx_bytes: u64,
}
