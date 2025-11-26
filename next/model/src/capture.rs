use crate::chip::{ChipId, ChipKind};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[async_trait]
pub trait CaptureSender: Send + Sync {
    async fn create_capture(&self, create: CaptureCreate) -> anyhow::Result<()>;
    fn capture_packet(&self, chip_id: ChipId, direction: Direction, packet: Bytes);
}

/// Direction of the packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Packet was sent from the device.
    Sent,
    /// Packet was received by the device.
    Received,
    /// Direction is unknown.
    Unknown,
}

/// Parameters for creating a new capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureCreate {
    /// The ID of the chip to capture.
    pub chip_id: ChipId,
    /// The kind of chip.
    pub chip_kind: ChipKind,
    /// The name of the device.
    pub device_name: String,
    /// Whether capture should be enabled by default.
    pub default_enabled: bool,
    /// Optional flag to be updated when capture status changes.
    #[serde(skip)]
    pub enabled_flag: Arc<AtomicBool>,
}

/// Actions that can be performed on a capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CaptureAction {
    /// Capture a packet.
    CapturePacket { chip_id: ChipId, direction: Direction, bytes: Bytes },
    /// Get a specific capture.
    Get { chip_id: ChipId },
    /// Patch a capture (e.g., enable/disable).
    Patch { chip_id: ChipId, enabled: bool },
    /// Create a new capture.
    Create { chip_id: ChipId, chip_kind: ChipKind, device_name: String },
    /// Delete a capture.
    Delete { chip_id: ChipId },
    /// Set default capture state for new captures.
    SetDefaultCapture { enabled: bool },
    /// Set default capture directory.
    SetCaptureDirectory { path: PathBuf },
}

/// Result of a capture action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CaptureActionResult {
    /// Action was successful.
    Success,
    /// Result of a Get action.
    Get(Option<CaptureInfo>),
    /// An error occurred.
    Error(String),
}

/// Information about a capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureInfo {
    /// The ID of the chip.
    pub chip_id: ChipId,
    /// The kind of chip.
    pub chip_kind: ChipKind,
    /// The name of the device.
    pub device_name: String,
    /// Whether capture is enabled.
    pub enabled: bool,
    /// Number of records written.
    pub records_written: u64,
    /// Number of bytes written.
    pub bytes_written: u64,
}
