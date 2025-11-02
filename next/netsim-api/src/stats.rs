// Rust definitions for statistics related structures
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimRadioStats {
    pub id: u32,
    pub name: String,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
}
