// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Device REST API data transfer models.

pub use netsim_model::{
    Device, DeviceAddChip, DeviceChipCreate, DeviceConfig, DeviceCreate, DeviceId, DeviceInfo,
    DeviceUpdate, PoseUpdate,
};

pub use crate::ListDeviceResponse;

impl From<Vec<Device>> for ListDeviceResponse {
    fn from(devices: Vec<Device>) -> Self {
        Self { devices }
    }
}
