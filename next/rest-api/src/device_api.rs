// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Device REST API data transfer models.

pub use netsim_model::{
    Device, DeviceAddChip, DeviceChipCreate, DeviceConfig, DeviceCreate, DeviceId, DeviceInfo,
    DeviceUpdate, PoseUpdate,
};
use serde::{Deserialize, Serialize};

/// Response body for listing simulated devices (`GET /v1/devices`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListDeviceResponse {
    pub devices: Vec<Device>,
}

impl From<Vec<Device>> for ListDeviceResponse {
    fn from(devices: Vec<Device>) -> Self {
        Self { devices }
    }
}
