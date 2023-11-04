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

use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::model::Chip as ProtoChip;

use crate::echip::{ble_beacon, bluetooth, mocked, wifi};

use std::rc::Rc;

/// Parameter for each constructor of Emulated Chips
pub enum CreateParam {
    BleBeacon(ble_beacon::CreateParams),
    Bluetooth(bluetooth::CreateParams),
    Wifi(wifi::CreateParams),
    Mock(mocked::CreateParams),
}

// TODO: Factory trait to include start, stop, and add
/// EmulatedChip is a trait that provides interface between the generic Chip
/// and Radio specific library (rootcanal, libslirp, pica).
pub trait EmulatedChip {
    /// This is the main entry for incoming host-to-controller packets
    /// from virtual devices called by the transport module. The format of the
    /// packet depends on the emulated chip kind:
    /// * Bluetooth - packet is H4 HCI format
    /// * Wi-Fi - packet is Radiotap format
    /// * UWB - packet is UCI format
    /// * NFC - packet is NCI format
    fn handle_request(&self, packet: &[u8]);

    /// Reset the internal state of the emulated chip for the virtual device.
    /// The transmitted and received packet count will be set to 0 and the chip
    /// shall be in the enabled state following a call to this function.
    fn reset(&self);

    /// Return the Chip model protobuf from the emulated chip. This is part of
    /// the Frontend API.
    fn get(&self) -> ProtoChip;

    /// Patch the state of the emulated chip. For example enable/disable the
    /// chip's host-to-controller packet processing. This is part of the
    /// Frontend API
    fn patch(&self, chip: ProtoChip);

    /// Returns the kind of the emulated chip.
    fn get_kind(&self) -> ProtoChipKind;
}

/// This is called when the transport module receives a new packet stream
/// connection from a virtual device.
pub fn new(create_param: CreateParam) -> Rc<dyn EmulatedChip> {
    match create_param {
        CreateParam::BleBeacon(params) => ble_beacon::new(params),
        CreateParam::Bluetooth(params) => bluetooth::new(params),
        CreateParam::Wifi(params) => wifi::new(params),
        CreateParam::Mock(params) => mocked::new(params),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_echip_new() {
        let mock_param = CreateParam::Mock(mocked::CreateParams {});
        let echip = new(mock_param);
        assert_eq!(echip.get_kind(), ProtoChipKind::UNSPECIFIED);
        assert_eq!(echip.get(), ProtoChip::new());
    }
}
