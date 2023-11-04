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

use crate::devices::chip::{ChipIdentifier, FacadeIdentifier};
use crate::devices::device::DeviceIdentifier;
use crate::echip::EmulatedChip;

use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::model::ChipCreate as ChipCreateProto;

use std::rc::Rc;

/// Parameters for creating BleBeacon chips
pub struct CreateParams {
    device_id: DeviceIdentifier,
    device_name: String,
    chip_id: ChipIdentifier,
    chip_proto: ChipCreateProto,
}

/// BleBeacon struct will keep track of facade_id
pub struct BleBeacon {
    facade_id: FacadeIdentifier,
}

impl EmulatedChip for BleBeacon {
    fn handle_request(&self, packet: &[u8]) {
        todo!();
    }

    fn reset(&self) {
        todo!();
    }

    fn get(&self) -> ProtoChip {
        todo!();
    }

    fn patch(&self, chip: ProtoChip) {
        todo!();
    }

    fn get_kind(&self) -> ProtoChipKind {
        ProtoChipKind::BLUETOOTH_BEACON
    }
}

impl Drop for BleBeacon {
    /// At drop, Remove the emulated chip from the virtual device. No further calls will
    /// be made on this emulated chip. This is called when the packet stream from
    /// the virtual device closes.
    fn drop(&mut self) {
        todo!();
    }
}

/// Create a new Emulated BleBeacon Chip
pub fn new(params: CreateParams) -> Rc<dyn EmulatedChip> {
    todo!();
}
