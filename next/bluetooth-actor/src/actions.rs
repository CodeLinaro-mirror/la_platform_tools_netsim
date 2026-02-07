// Copyright 2025 The Android Open Source Project

use netsim_model::{chip::ChipId, stats::NetsimRadioStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Actions that can be performed on the Bluetooth actor.
pub enum BluetoothAction {
    /// Resets the chip with the given ID.
    Reset {
        /// The ID of the chip to reset.
        id: ChipId,
    },
    /// Retrieves statistics for all chips.
    GetStatistics,
    /// Retrieves the total number of chips for testing.
    GetCountForTesting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The result of a Bluetooth action.
pub enum BluetoothActionResult {
    /// The action succeeded with no return value.
    Success,
    /// The action returned statistics.
    Statistics(Box<[NetsimRadioStats]>),
    /// The action returned a count.
    Count(usize),
}
