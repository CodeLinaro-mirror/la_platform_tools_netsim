// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use pica::{Handle, RangingEstimator, RangingMeasurement};

use crate::devices::{chip::ChipIdentifier, devices_handler::get_device};
use crate::ranging::{compute_range_azimuth_elevation, Pose};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

// Initially a single HashMap might grow into a Struct to allow
// for more fields.
type State = HashMap<Handle, ChipIdentifier>;

#[derive(Clone)]
pub struct SharedState(Arc<Mutex<State>>);

impl SharedState {
    pub fn new() -> Self {
        SharedState(Arc::new(Mutex::new(State::new())))
    }

    fn lock(&self) -> MutexGuard<State> {
        self.0.lock().expect("Poisoned SharedState lock")
    }

    pub fn get_chip_id(&self, pica_id: &Handle) -> anyhow::Result<ChipIdentifier> {
        self.lock().get(pica_id).ok_or(anyhow::anyhow!("pica_id: {pica_id} not in State")).copied()
    }

    pub fn insert(&self, pica_id: Handle, chip_id: ChipIdentifier) {
        self.lock().insert(pica_id, chip_id);
    }

    pub fn remove(&self, pica_id: &Handle) {
        self.lock().remove(pica_id);
    }
}

// Netsim's UwbRangingEstimator
pub struct UwbRangingEstimator {
    shared_state: SharedState,
}

impl UwbRangingEstimator {
    pub fn new(shared_state: SharedState) -> Self {
        UwbRangingEstimator { shared_state }
    }
}

impl RangingEstimator for UwbRangingEstimator {
    fn estimate(&self, a: &Handle, b: &Handle) -> anyhow::Result<RangingMeasurement> {
        // Use the Handle to obtain the positions and orientation information in netsim
        // and perform compute_range_azimuth_elevation
        let a_device = get_device(self.shared_state.get_chip_id(a)?)?;
        let b_device = get_device(self.shared_state.get_chip_id(b)?)?;
        let (a_p, a_o) = (a_device.position, a_device.orientation);
        let (b_p, b_o) = (b_device.position, b_device.orientation);
        let a_pose = Pose::new(a_p.x, a_p.y, a_p.z, a_o.yaw, a_o.pitch, a_o.roll);
        let b_pose = Pose::new(b_p.x, b_p.y, b_p.z, b_o.yaw, b_o.pitch, b_o.roll);
        let (range, azimuth, elevation) = compute_range_azimuth_elevation(&a_pose, &b_pose)?;
        Ok(RangingMeasurement { range, azimuth, elevation })
    }
}
