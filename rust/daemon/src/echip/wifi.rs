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

use crate::devices::chip::FacadeIdentifier;
use crate::ffi::ffi_wifi;
use crate::wifi::medium;
use crate::{
    devices::device::DeviceIdentifier,
    echip::{EmulatedChip, SharedEmulatedChip},
};
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::config::WiFi as WiFiConfig;
use netsim_proto::model::chip::Radio;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};
use protobuf::{Message, MessageField};

use std::sync::Arc;

/// Parameters for creating Wifi chips
pub struct CreateParams {}

/// Wifi struct will keep track of facade_id
pub struct Wifi {
    facade_id: FacadeIdentifier,
}

impl EmulatedChip for Wifi {
    fn handle_request(&self, packet: &[u8]) {
        if crate::config::get_dev() {
            let _ = medium::parse_hwsim_cmd(packet);
        }
        ffi_wifi::handle_wifi_request(self.facade_id, &packet.to_vec());
    }

    fn reset(&self) {
        ffi_wifi::wifi_reset(self.facade_id);
    }

    fn get(&self) -> ProtoChip {
        let radio_bytes = ffi_wifi::wifi_get_cxx(self.facade_id);
        let wifi_proto = Radio::parse_from_bytes(&radio_bytes).unwrap();
        let mut chip_proto = ProtoChip::new();
        chip_proto.mut_wifi().clone_from(&wifi_proto);
        chip_proto
    }

    fn patch(&self, chip: &ProtoChip) {
        let radio_bytes = chip.wifi().write_to_bytes().unwrap();
        ffi_wifi::wifi_patch_cxx(self.facade_id, &radio_bytes);
    }

    fn remove(&self) {
        ffi_wifi::wifi_remove(self.facade_id);
    }

    fn get_stats(&self, duration_secs: u64) -> Vec<ProtoRadioStats> {
        let mut stats_proto = ProtoRadioStats::new();
        stats_proto.set_duration_secs(duration_secs);
        stats_proto.set_kind(netsim_radio_stats::Kind::WIFI);
        let chip_proto = self.get();
        if chip_proto.has_wifi() {
            stats_proto.set_tx_count(chip_proto.wifi().tx_count);
            stats_proto.set_rx_count(chip_proto.wifi().rx_count);
        }
        vec![stats_proto]
    }

    fn get_kind(&self) -> ProtoChipKind {
        ProtoChipKind::WIFI
    }

    fn get_facade_id(&self) -> FacadeIdentifier {
        self.facade_id
    }
}

/// Create a new Emulated Wifi Chip
pub fn new(_params: &CreateParams, device_id: DeviceIdentifier) -> SharedEmulatedChip {
    let facade_id = ffi_wifi::wifi_add(device_id);
    let echip = Wifi { facade_id };
    Arc::new(Box::new(echip))
}

/// Starts the WiFi service.
pub fn wifi_start(config: &MessageField<WiFiConfig>) {
    if crate::config::get_dev() {
        medium::test_parse_hwsim_cmd();
    }
    let proto_bytes = config.as_ref().unwrap_or_default().write_to_bytes().unwrap();
    ffi_wifi::wifi_start(&proto_bytes);
}

/// Stops the WiFi service.
pub fn wifi_stop() {
    ffi_wifi::wifi_stop();
}
