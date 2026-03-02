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
    /// Starts ranging for the given chip and session.
    #[cfg(any(test, feature = "testing"))]
    StartRanging {
        /// The ID of the chip to start ranging on.
        id: ChipId,
        /// The ID of the session to start ranging for.
        session_id: u32,
    },
    /// Stops ranging for the given chip and session.
    #[cfg(any(test, feature = "testing"))]
    StopRanging {
        /// The ID of the chip to stop ranging on.
        id: ChipId,
        /// The ID of the session to stop ranging for.
        session_id: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The result of a UWB action.
pub enum UwbActionResult {
    /// The action succeeded with no return value.
    Success,
    /// The action returned statistics.
    Statistics(Box<[NetsimRadioStats]>),
}
