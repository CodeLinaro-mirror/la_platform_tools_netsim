// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::ChipId;

/// Parameters for creating a Link.
#[derive(Debug, Clone)]
pub struct LinkCreate {
    pub sender: ChipId,
    pub receiver: ChipId,
    pub rssi: i8,
}
