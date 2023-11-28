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

use crate::bluetooth::{ble_beacon_add, ble_beacon_get, ble_beacon_patch, ble_beacon_remove};
use crate::devices::chip::{ChipIdentifier, FacadeIdentifier};
use crate::devices::device::DeviceIdentifier;
use crate::echip::{EmulatedChip, SharedEmulatedChip};
use crate::ffi::ffi_bluetooth;

use log::error;
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::model::ChipCreate as ChipCreateProto;

use std::sync::Arc;

/// Parameters for creating BleBeacon chips
pub struct CreateParams {
    pub device_name: String,
    pub chip_id: ChipIdentifier,
    pub chip_proto: ChipCreateProto,
}

/// BleBeacon struct will keep track of facade_id
pub struct BleBeacon {
    facade_id: FacadeIdentifier,
    chip_id: ChipIdentifier,
}

impl EmulatedChip for BleBeacon {
    fn handle_request(&self, packet: &[u8]) {
        ffi_bluetooth::handle_bt_request(self.facade_id, packet[0], &packet[1..].to_vec())
    }

    fn reset(&self) {
        ffi_bluetooth::bluetooth_reset(self.facade_id);
    }

    fn get(&self) -> ProtoChip {
        let mut chip_proto = ProtoChip::new();
        match ble_beacon_get(self.chip_id, self.facade_id) {
            Ok(beacon_proto) => chip_proto.mut_ble_beacon().clone_from(&beacon_proto),
            Err(err) => error!("{err:?}"),
        }
        chip_proto
    }

    fn patch(&self, chip: &ProtoChip) {
        if let Err(err) = ble_beacon_patch(self.facade_id, self.chip_id, chip.ble_beacon()) {
            error!("{err:?}");
        }
    }

    fn get_kind(&self) -> ProtoChipKind {
        ProtoChipKind::BLUETOOTH_BEACON
    }

    fn get_facade_id(&self) -> FacadeIdentifier {
        self.facade_id
    }
}

impl Drop for BleBeacon {
    /// At drop, Remove the emulated chip from the virtual device. No further calls will
    /// be made on this emulated chip. This is called when the packet stream from
    /// the virtual device closes.
    fn drop(&mut self) {
        if let Err(err) = ble_beacon_remove(self.chip_id, self.facade_id) {
            error!("{err:?}");
        }
    }
}

/// Create a new Emulated BleBeacon Chip
pub fn new(params: &CreateParams, device_id: DeviceIdentifier) -> SharedEmulatedChip {
    match ble_beacon_add(device_id, params.device_name.clone(), params.chip_id, &params.chip_proto)
    {
        Ok(facade_id) => Arc::new(Box::new(BleBeacon { facade_id, chip_id: params.chip_id })),
        Err(err) => {
            error!("{err:?}");
            Arc::new(Box::new(BleBeacon { facade_id: u32::MAX, chip_id: u32::MAX }))
        }
    }
}
