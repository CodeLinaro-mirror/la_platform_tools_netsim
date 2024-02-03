// Copyright 2023 Google LLC
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

use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::NetsimRadioStats as ProtoRadioStats;

use crate::devices::chip::ChipIdentifier;

use super::{EmulatedChip, SharedEmulatedChip};

#[cfg(feature = "cuttlefish")]
#[allow(unused_imports)]
use pica::Pica;

type PicaIdentifier = u32;

/// Parameters for creating UWB chips
pub struct CreateParams {
    mac_address: u64,
}

/// UWB struct will keep track of pica_id
pub struct Uwb {
    pica_id: PicaIdentifier,
}

impl EmulatedChip for Uwb {
    fn handle_request(&self, packet: &[u8]) {
        todo!()
    }

    fn reset(&mut self) {
        todo!()
    }

    fn get(&self) -> ProtoChip {
        todo!()
    }

    fn patch(&mut self, chip: &ProtoChip) {
        todo!()
    }

    fn remove(&mut self) {
        todo!()
    }

    fn get_stats(&self, duration_secs: u64) -> Vec<ProtoRadioStats> {
        todo!()
    }
}

pub fn new(create_params: &CreateParams, chip_id: ChipIdentifier) -> SharedEmulatedChip {
    todo!()
}

#[cfg(test)]
mod tests {
    #[ignore = "TODO: Perform new and check SharedEmulatedChip"]
    #[test]
    fn test_new() {
        todo!();
    }

    #[ignore = "TODO: Check if get properly returns a Chip protobuf"]
    #[test]
    fn test_uwb_get() {
        todo!()
    }

    #[ignore = "TODO: Check if reset properly sets the state to true"]
    #[test]
    fn test_uwb_reset() {
        todo!()
    }

    #[ignore = "TODO: Check if get_stats returns the proper ProtoRadioStats"]
    #[test]
    fn test_get_stats() {
        todo!()
    }
}
