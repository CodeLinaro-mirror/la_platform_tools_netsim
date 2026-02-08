// Copyright 2026 The Android Open Source Project

use netsim_model::{chip::ChipId, stats::NetsimRadioStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Actions that can be performed on the UWB actor.
pub enum UwbAction {
    /// Resets the chip with the given ID.
    Reset {
        /// The ID of the chip to reset.
        id: ChipId,
    },
    /// Retrieves statistics for all chips.
    GetStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The result of a UWB action.
pub enum UwbActionResult {
    /// The action succeeded with no return value.
    Success,
    /// The action returned statistics.
    Statistics(Box<[NetsimRadioStats]>),
}
