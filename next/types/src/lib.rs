//! Netsim Common Types
//!
//! This crate contains common, low-level data structures used across Netsim crates.
//! By separating these types into their own crate, we avoid circular dependencies
//! and reduce compilation times.
//!
//! Major types included:
//! - `ChipInfo`: Information about a chip and its device.
//! - `ChipKind`: Enumeration of supported network technologies.

use serde::{Deserialize, Serialize};

/// Information about the chip and device provided during connection setup.
///
/// This struct is used during the handshake phase to identify the connecting chip
/// and its parent device.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChipInfo {
    /// The name of the chip (e.g., "bt-main").
    pub name: String,
    /// Detailed chip information, if available.
    pub chip: Option<Chip>,
    /// Detailed device information, if available.
    pub device_info: Option<DeviceInfo>,
}

impl ChipInfo {
    /// Creates a new ChipInfo for a specific chip kind.
    pub fn new<S: Into<String>>(name: S, kind: ChipKind) -> Self {
        let name = name.into();
        ChipInfo {
            device_info: Some(DeviceInfo::new(name.clone(), name.clone())),
            chip: Some(Chip::new(kind, name.clone())),
            name,
        }
    }

    /// Get the device name from the device_info, or a default value.
    pub fn device_name(&self) -> String {
        self.device_info.as_ref().map_or_else(|| "Unknown".to_string(), |d| d.name.clone())
    }
}

/// Details about a specific chip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chip {
    /// The kind of chip (Bluetooth, Wifi, etc.).
    pub kind: ChipKind,
    /// Unique identifier for the chip.
    pub id: String,
    /// Human-readable name of the chip.
    pub name: String,
    /// Manufacturer of the chip.
    pub manufacturer: String,
    /// Product name of the chip.
    pub product_name: String,
    /// Address of the chip (e.g. MAC address).
    pub address: String,
}

impl Chip {
    pub fn new<S: Into<String>>(kind: ChipKind, name: S) -> Self {
        let name = name.into();
        Chip {
            kind,
            id: name.clone(),
            name,
            manufacturer: "Netsim".to_string(),
            product_name: "Virtual Chip".to_string(),
            address: "".to_string(),
        }
    }
}

/// Details about the device hosting the chip.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceInfo {
    /// Human-readable name of the device.
    pub name: String,
    /// Unique identifier for the device.
    pub id: String,
    /// Path to the AVD directory, if applicable.
    pub avd_path: String,
}

impl DeviceInfo {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, id: S2) -> Self {
        DeviceInfo { name: name.into(), id: id.into(), avd_path: "".to_string() }
    }
}

/// The kind of network technology the chip supports.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum ChipKind {
    #[default]
    UNSPECIFIED,
    BLUETOOTH,
    WIFI,
    UWB,
    CELL,
}
