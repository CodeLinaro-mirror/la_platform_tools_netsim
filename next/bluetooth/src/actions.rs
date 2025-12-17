// Copyright 2025 The Android Open Source Project

use netsim_model::chip::ChipId;
use netsim_model::stats::NetsimRadioStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BluetoothAction {
    Reset { id: ChipId },
    GetStatistics,
    GetCountForTesting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BluetoothActionResult {
    Success,
    Statistics(Vec<NetsimRadioStats>),
    Count(usize),
}
