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

use super::chip::{rust_bluetooth_add, RustBluetoothChipCallbacks};
use crate::devices::devices_handler::add_chip;
use crate::ffi::{generate_advertising_packet, RustBluetoothChip};
use cxx::{let_cxx_string, UniquePtr};
use frontend_proto::common::ChipKind;
use lazy_static::lazy_static;
use log::{error, info, warn};
use std::alloc::System;
use std::sync::{Mutex, RwLock};
use std::{collections::HashMap, ptr::null};

// A singleton that contains a hash map from chip id to RustBluetoothChip.
// It's used by `BeaconChip` to access `RustBluetoothChip` to call send_link_layer_packet().
lazy_static! {
    static ref BLUETOOTH_BEACON_CHIPS: RwLock<HashMap<u32, Mutex<UniquePtr<RustBluetoothChip>>>> =
        RwLock::new(HashMap::new());
}

/// BeaconChip class.
pub struct BeaconChip {
    chip_id: u32,
    address: String,
}

impl BeaconChip {
    pub fn new(chip_id: u32, address: String) -> Self {
        BeaconChip { chip_id, address }
    }

    pub fn send_link_layer_packet(&mut self, packet: &[u8], packet_type: u8, tx_power: i8) {
        let binding = BLUETOOTH_BEACON_CHIPS.read().unwrap();
        if let Some(rust_bluetooth_chip) = binding.get(&self.chip_id) {
            rust_bluetooth_chip.lock().unwrap().pin_mut().send_link_layer_packet(
                packet,
                packet_type,
                tx_power,
            );
        } else {
            warn!("Failed to get RustBluetoothChip for unknown chip id {}", self.chip_id);
        };
    }
}

impl RustBluetoothChipCallbacks for BeaconChip {
    fn tick(&mut self) {
        // TODO: Implement by following the example of Beacon::Tick() in
        //       packages/modules/Bluetooth/tools/rootcanal/model/devices/beacon.cc.
    }

    fn receive_link_layer_packet(
        &mut self,
        source_address: String,
        destination_address: String,
        packet_type: u8,
        packet: &[u8],
    ) {
        // TODO: Implement by following the example of Beacon::ReceiveLinkLayerPacket()
        //       in packages/modules/Bluetooth/tools/rootcanal/model/devices/beacon.cc.
    }
}

/// Add a beacon device in rootcanal.
///
/// Called by `devices/chip.rs`.
///
/// Similar to `bluetooth_add()`.
pub fn bluetooth_beacon_add(
    device_id: u32,
    chip_id: u32,
    device_type: String,
    address: String,
) -> u32 {
    let mut beacon_chip = BeaconChip::new(chip_id, address.clone());
    let callbacks: Box<dyn RustBluetoothChipCallbacks> = Box::new(beacon_chip);
    let mut add_rust_device_result = rust_bluetooth_add(device_id, callbacks, device_type, address);
    let rust_chip = add_rust_device_result.rust_chip;
    let facade_id = add_rust_device_result.facade_id;

    info!("Creating HCI facade {} for device {} chip {}", facade_id, device_id, chip_id);
    BLUETOOTH_BEACON_CHIPS.write().unwrap().insert(chip_id, Mutex::new(rust_chip));
    facade_id
}

// TODO: Support removing Beacon.

/// Create a new beacon device. Used by CLI or web.
pub fn new_beacon(beacon_name: String, address: String) {
    // TODO: Support passing BluetoothBeacon and call patch_device().

    let device_guid = format!("{}_device_guid", &beacon_name);
    let device_name = format!("{}_device_name", &beacon_name);
    let chip_name = address; // Use chip_name to store address.
    let chip_manufacturer = format!("{}_chip_manufacturer", &beacon_name);
    let chip_product_name = format!("{}_chip_product_name", &beacon_name);
    let result = match add_chip(
        &device_guid,
        &device_name,
        ChipKind::BLUETOOTH_BEACON,
        &chip_name,
        &chip_manufacturer,
        &chip_product_name,
    ) {
        Ok(chip_result) => chip_result,
        Err(err) => {
            error!("{err}");
            return;
        }
    };
}
