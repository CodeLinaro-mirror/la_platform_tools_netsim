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
use crate::ffi::ffi_bluetooth;
use crate::{
    devices::device::DeviceIdentifier,
    echip::{EmulatedChip, SharedEmulatedChip},
};

use cxx::let_cxx_string;
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::config::Bluetooth as BluetoothConfig;
use netsim_proto::configuration::Controller as RootcanalController;
use netsim_proto::model::chip::Bluetooth as ProtoBluetooth;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};
use protobuf::{Message, MessageField};

use std::sync::{Arc, Mutex};

static ECHIP_BT_MUTEX: Mutex<()> = Mutex::new(());

/// Parameters for creating Bluetooth chips
pub struct CreateParams {
    pub address: String,
    pub bt_properties: Option<MessageField<RootcanalController>>,
}

/// Bluetooth struct will keep track of facade_id
pub struct Bluetooth {
    facade_id: FacadeIdentifier,
}

impl EmulatedChip for Bluetooth {
    fn handle_request(&self, packet: &[u8]) {
        ffi_bluetooth::handle_bt_request(self.facade_id, packet[0], &packet[1..].to_vec())
    }

    fn reset(&self) {
        ffi_bluetooth::bluetooth_reset(self.facade_id);
    }

    fn get(&self) -> ProtoChip {
        let bluetooth_bytes = ffi_bluetooth::bluetooth_get_cxx(self.facade_id);
        let bt_proto = ProtoBluetooth::parse_from_bytes(&bluetooth_bytes).unwrap();
        let mut chip_proto = ProtoChip::new();
        chip_proto.mut_bt().clone_from(&bt_proto);
        chip_proto
    }

    fn patch(&self, chip: &ProtoChip) {
        let bluetooth_bytes = chip.bt().write_to_bytes().unwrap();
        ffi_bluetooth::bluetooth_patch_cxx(self.facade_id, &bluetooth_bytes);
    }

    fn remove(&self) {
        // Lock to protect id_to_chip_info_ table in C++
        let _unused = ECHIP_BT_MUTEX.lock().expect("Failed to acquire lock on ECHIP_BT_MUTEX");
        ffi_bluetooth::bluetooth_remove(self.facade_id);
    }

    fn get_stats(&self, duration_secs: u64) -> Vec<ProtoRadioStats> {
        // Construct NetsimRadioStats for BLE and Classic.
        let mut ble_stats_proto = ProtoRadioStats::new();
        ble_stats_proto.set_duration_secs(duration_secs);
        let mut classic_stats_proto = ble_stats_proto.clone();

        // Obtain the Chip Information with get()
        let chip_proto = self.get();
        if chip_proto.has_bt() {
            // Setting values for BLE Radio Stats
            ble_stats_proto.set_kind(netsim_radio_stats::Kind::BLUETOOTH_LOW_ENERGY);
            ble_stats_proto.set_tx_count(chip_proto.bt().low_energy.tx_count);
            ble_stats_proto.set_rx_count(chip_proto.bt().low_energy.rx_count);
            // Setting values for Classic Radio Stats
            classic_stats_proto.set_kind(netsim_radio_stats::Kind::BLUETOOTH_CLASSIC);
            classic_stats_proto.set_tx_count(chip_proto.bt().classic.tx_count);
            classic_stats_proto.set_rx_count(chip_proto.bt().classic.rx_count);
        }
        vec![ble_stats_proto, classic_stats_proto]
    }

    fn get_kind(&self) -> ProtoChipKind {
        ProtoChipKind::BLUETOOTH
    }

    fn get_facade_id(&self) -> FacadeIdentifier {
        self.facade_id
    }
}

/// Create a new Emulated Bluetooth Chip
pub fn new(create_params: &CreateParams, device_id: DeviceIdentifier) -> SharedEmulatedChip {
    // Lock to protect id_to_chip_info_ table in C++
    let _unused = ECHIP_BT_MUTEX.lock().expect("Failed to acquire lock on ECHIP_BT_MUTEX");
    let_cxx_string!(cxx_address = create_params.address.clone());
    let proto_bytes = match &create_params.bt_properties {
        Some(properties) => properties.write_to_bytes().unwrap(),
        None => Vec::new(),
    };
    let facade_id = ffi_bluetooth::bluetooth_add(device_id, &cxx_address, &proto_bytes);
    let echip = Bluetooth { facade_id };
    Arc::new(Box::new(echip))
}

/// Starts the Bluetooth service.
pub fn bluetooth_start(
    config: &MessageField<BluetoothConfig>,
    instance_num: u16,
    disable_address_reuse: bool,
) {
    let proto_bytes = config.as_ref().unwrap_or_default().write_to_bytes().unwrap();
    ffi_bluetooth::bluetooth_start(&proto_bytes, instance_num, disable_address_reuse);
}

/// Stops the Bluetooth service.
pub fn bluetooth_stop() {
    ffi_bluetooth::bluetooth_stop();
}
