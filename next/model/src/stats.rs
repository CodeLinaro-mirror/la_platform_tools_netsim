// Rust definitions for statistics related structures
use serde::{Deserialize, Serialize};

/// Represents an invalid packet with a reason and description.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InvalidPacket {
    pub reason: String,
    pub description: String,
    // Packet content is intentionally omitted from the model to avoid large payloads in stats
}

/// Represents the statistics for a radio.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimRadioStats {
    /// The unique ID of the chip.
    pub id: u32,
    /// The name of the chip.
    pub name: String,
    /// The kind of the radio (e.g. BLUETOOTH_LOW_ENERGY, WIFI).
    pub kind: crate::ChipKind,
    /// Duration of the stats session in seconds.
    pub duration_secs: u64,
    /// Number of packets transmitted.
    pub tx_count: u64,
    /// Number of packets received.
    pub rx_count: u64,
    /// Number of bytes transmitted.
    pub tx_bytes: u64,
    /// Number of bytes received.
    pub rx_bytes: u64,
    /// List of invalid packets encountered.
    // TODO: Cap this vector to avoid unbounded growth (e.g. max 50 items).
    pub invalid_packets: Vec<InvalidPacket>,
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
    /// The version of the device.
    pub version: Option<String>,
    /// The SDK version of the device.
    pub sdk_version: Option<String>,
    /// The build ID of the device.
    pub build_id: Option<String>,
    /// The variant of the device.
    pub variant: Option<String>,
    /// The architecture of the device.
    pub arch: Option<String>,
}

/// Represents the statistics for the frontend.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimFrontendStats {
    pub get_version: u32,
    pub create_device: u32,
    pub delete_chip: u32,
    pub patch_device: u32,
    pub reset: u32,
    pub list_device: u32,
    pub subscribe_device: u32,
    pub patch_capture: u32,
    pub list_capture: u32,
    pub get_capture: u32,
}
