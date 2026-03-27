// Copyright 2026 Google LLC

//! Implementation of [RangingEstimator] for use with [pica].

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use pica::{Handle, RangingEstimator, RangingMeasurement};
use tracing::warn;

use crate::{
    ranging::{compute_range_azimuth_elevation, Pose},
    uwb_actor::UwbChipState,
};

/// UWB Ranging Estimator that uses shared chip state to calculate ranging
/// measurements.
pub(crate) struct UwbRangingEstimator {
    shared_chips: Arc<RwLock<HashMap<Handle, UwbChipState>>>,
}

impl UwbRangingEstimator {
    pub fn new(shared_chips: Arc<RwLock<HashMap<Handle, UwbChipState>>>) -> Self {
        Self { shared_chips }
    }
}

impl RangingEstimator for UwbRangingEstimator {
    fn estimate(&self, a_handle: &Handle, b_handle: &Handle) -> Option<RangingMeasurement> {
        let chips = self.shared_chips.read().unwrap();
        let a_state = chips.get(a_handle)?;
        let b_state = chips.get(b_handle)?;

        if !a_state.chip.is_uwb_enabled() || !b_state.chip.is_uwb_enabled() {
            return None;
        }

        let a_pose = Pose::from(&a_state.chip.pose);
        let b_pose = Pose::from(&b_state.chip.pose);

        match compute_range_azimuth_elevation(&a_pose, &b_pose) {
            Ok((range, azimuth, elevation)) => {
                // TODO(b/484364478): sampled ranging support
                Some(RangingMeasurement { range: range as u16, azimuth, elevation })
            }
            Err(e) => {
                warn!("Failed to estimate ranging measurement: {}", e);
                None
            }
        }
    }
}
