// Copyright 2025 Google LLC
//=============================================================================
// src/initial_info.rs - Data models for initial connection handshake
//=============================================================================

use serde::{Deserialize, Serialize};

/// Information about the chip and device provided during connection setup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipInfo {
    pub name: String,
    pub chip: Option<Chip>,
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
    pub kind: ChipKind,
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub product_name: String,
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
        }
    }
}

/// Details about the device hosting the chip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub id: String,
}

impl DeviceInfo {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, id: S2) -> Self {
        DeviceInfo { name: name.into(), id: id.into() }
    }
}

/// The kind of network technology the chip supports.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChipKind {
    UNSPECIFIED,
    BLUETOOTH,
    WIFI,
    UWB,
    CELL,
}
