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

use std::sync::Arc;

use bytes::Bytes;

use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::NetsimRadioStats as ProtoRadioStats;

use crate::{
    devices::chip::ChipIdentifier,
    echip::{ble_beacon, mocked},
};

pub type SharedEmulatedChip = Arc<Box<dyn EmulatedChip + Send + Sync>>;

#[cfg(not(test))]
use crate::echip::{bluetooth, wifi};

// TODO(b/278268690): Add Pica Library to goldfish build
#[cfg(feature = "cuttlefish")]
use crate::echip::uwb;

/// Parameter for each constructor of Emulated Chips
#[allow(clippy::large_enum_variant, dead_code)]
pub enum CreateParam {
    BleBeacon(ble_beacon::CreateParams),
    #[cfg(not(test))]
    Bluetooth(bluetooth::CreateParams),
    #[cfg(not(test))]
    Wifi(wifi::CreateParams),
    // TODO(b/278268690): Add Pica Library to goldfish build
    #[cfg(feature = "cuttlefish")]
    Uwb(uwb::CreateParams),
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
    fn handle_request(&self, packet: &Bytes);

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
    fn patch(&self, chip: &ProtoChip);

    /// Return the NetsimRadioStats protobuf from the emulated chip. This is
    /// part of NetsimStats protobuf.
    fn get_stats(&self, duration_secs: u64) -> Vec<ProtoRadioStats>;
}

/// This is called when the transport module receives a new packet stream
/// connection from a virtual device.
pub fn new(create_param: &CreateParam, chip_id: ChipIdentifier) -> SharedEmulatedChip {
    // Based on create_param, construct SharedEmulatedChip.
    match create_param {
        CreateParam::BleBeacon(params) => ble_beacon::new(params, chip_id),
        #[cfg(not(test))]
        CreateParam::Bluetooth(params) => bluetooth::new(params, chip_id),
        #[cfg(not(test))]
        CreateParam::Wifi(params) => wifi::new(params, chip_id),
        // TODO(b/278268690): Add Pica Library to goldfish build
        #[cfg(feature = "cuttlefish")]
        CreateParam::Uwb(params) => uwb::new(params, chip_id),
        CreateParam::Mock(params) => mocked::new(params, chip_id),
    }
}

// TODO(b/309529194):
// 1. Create Mock echip, patch and get
// 2. Create Mock echip, patch and reset
#[cfg(test)]
mod tests {
    #[test]
    fn test_echip_new() {
        // TODO
    }
}
