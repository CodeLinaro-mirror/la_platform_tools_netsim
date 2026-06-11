// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use device_actor::DeviceClient;
use netsim_model::ChipId;

pub struct ChipState {
    pub device_id: device_api::DeviceId,
    pub enabled: bool,
}

pub struct NfcActor {
    pub device_client: DeviceClient,
    pub active_chips: HashMap<ChipId, ChipState>,
}

impl NfcActor {
    pub fn new(device_client: DeviceClient) -> Self {
        Self { device_client, active_chips: HashMap::new() }
    }
}
