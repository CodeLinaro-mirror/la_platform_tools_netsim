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

/// Represents the statistics for a device.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimDeviceStats {
    /// The unique ID of the device.
    pub device_id: Option<u32>,
    /// The name of the device.
    pub name: Option<String>,
    /// The kind of the device (e.g., "pixel6").
    pub kind: Option<String>,
    // Add other fields as needed
}

/// Represents the statistics for the frontend.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimFrontendStats {
    // Add fields as needed, currently placeholder
}
