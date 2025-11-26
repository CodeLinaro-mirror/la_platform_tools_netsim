use netsim_model::chip::{ChipId, ChipKind};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Entity representing a packet capture for a specific chip.
///
/// This entity manages the state of a single packet capture session,
/// including whether it is enabled and the associated metadata.
/// It does not hold the actual writer, which is stored in the context
/// to allow shared access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureEntity {
    /// The ID of the chip being captured.
    pub chip_id: ChipId,
    /// The kind of chip (e.g., Bluetooth, Wifi).
    pub chip_kind: ChipKind,
    /// The name of the device.
    pub device_name: String,
    /// Whether capture is currently enabled.
    pub enabled: bool,
    /// Flag to be updated when capture status changes.
    /// This is used by the stream wrappers to check if capture is enabled
    /// without locking the actor.
    #[serde(skip)]
    pub enabled_flag: Arc<AtomicBool>,
}
