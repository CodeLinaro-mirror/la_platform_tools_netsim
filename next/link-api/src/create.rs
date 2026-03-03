// Copyright 2025 The Android Open Source Project

use netsim_model::ChipId;

/// Parameters for creating a Link.
#[derive(Debug, Clone)]
pub struct LinkCreate {
    pub sender: ChipId,
    pub receiver: ChipId,
    pub rssi: i8,
}
