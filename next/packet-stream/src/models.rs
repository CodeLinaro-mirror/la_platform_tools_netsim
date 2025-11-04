// Copyright 2025 Google LLC
//=============================================================================
// src/models.rs - Data models for PacketStream
//=============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipInfo {
    pub name: String,
    pub chip: Option<Chip>,
    pub device_info: Option<DeviceInfo>,
}

impl ChipInfo {
    /// Get the device name from the device_info, or a default value.
    pub fn device_name(&self) -> String {
        self.device_info.as_ref().map_or_else(|| "Unknown".to_string(), |d| d.name.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chip {
    pub kind: ChipKind,
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub product_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChipKind {
    Unspecified,
    Bluetooth,
    Wifi,
    Uwb,
}
