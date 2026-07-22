// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Netsim Common Types
//!
//! This crate contains common, low-level data structures used across Netsim
//! crates. By separating these types into their own crate, we avoid circular
//! dependencies and reduce compilation times.
//!
//! Major types included:
//! - `ChipInfo`: Information about a chip and its device.
//! - `ChipKind`: Enumeration of supported network technologies.

use serde::{Deserialize, Serialize};

/// Information about the chip and device provided during connection setup.
///
/// This struct is used during the handshake phase to identify the connecting
/// chip and its parent device.
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
    /// SIM card type (0 = No SIM, 1 = Normal SIM, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sim_type: Option<i32>,
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
            sim_type: None,
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
    /// Identifier for device kind e.g. EMULATOR, CUTTLEFISH, BUMBLE, etc
    #[serde(default)]
    pub kind: String,
    /// Version info as applicable e.g. Android emulator version 34.1.15.0, etc
    #[serde(default)]
    pub version: String,
    /// SDK version info as applicable e.g. 33, 34, etc
    #[serde(default)]
    pub sdk_version: String,
    /// Build ID e.g. TE1A.220922.034, UQ1A.231205.015, etc
    #[serde(default)]
    pub build_id: String,
    /// Model/variant e.g. sdk_gphone_x86_64-userdebug, cf_x86_64_phone-user,
    /// etc
    #[serde(default)]
    pub variant: String,
    /// CPU architecture e.g. x86_64, arm64-v8a, etc
    #[serde(default)]
    pub arch: String,
    /// Path to the AVD directory, if applicable.
    pub avd_path: String,
}

impl DeviceInfo {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, id: S2) -> Self {
        DeviceInfo { name: name.into(), id: id.into(), ..Default::default() }
    }
}

/// The kind of network technology the chip supports.
///
/// This enumeration is used to distinguish between different types of simulated
/// radios and to route packets to the correct handlers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[allow(non_camel_case_types)]
pub enum ChipKind {
    #[default]
    UNSPECIFIED,
    BLUETOOTH,
    WIFI,
    UWB,
    NFC,
    CELLULAR,
    CELLULAR_DATA,
    ETHERNET,
}

impl std::str::FromStr for ChipKind {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "UNSPECIFIED" => Ok(ChipKind::UNSPECIFIED),
            "BLUETOOTH" => Ok(ChipKind::BLUETOOTH),
            "WIFI" => Ok(ChipKind::WIFI),
            "UWB" => Ok(ChipKind::UWB),
            "NFC" => Ok(ChipKind::NFC),
            "CELLULAR" | "MODEM" => Ok(ChipKind::CELLULAR),
            "CELLULAR_DATA" => Ok(ChipKind::CELLULAR_DATA),
            "ETHERNET" => Ok(ChipKind::ETHERNET),
            _ => Err(format!("invalid chip kind: {}", s)),
        }
    }
}
