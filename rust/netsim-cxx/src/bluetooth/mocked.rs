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

use crate::bluetooth::{BeaconChip, BEACON_CHIPS};
use crate::devices::chip::{ChipIdentifier, FacadeIdentifier};
use crate::devices::device::{AddChipResult, DeviceIdentifier};
use frontend_proto::model::chip::{Bluetooth, BluetoothBeacon};
use frontend_proto::model::chip_create::Chip as Builtin;
use frontend_proto::model::{ChipCreate, DeviceCreate};
use lazy_static::lazy_static;
use log::info;
use std::sync::Mutex;
use std::sync::RwLock;
use std::{collections::HashMap, ptr::null};

lazy_static! {
    static ref IDS: RwLock<FacadeIds> = RwLock::new(FacadeIds::new());
}

struct FacadeIds {
    pub current_id: u32,
}

impl FacadeIds {
    fn new() -> Self {
        FacadeIds { current_id: 0 }
    }
}

pub fn handle_bluetooth_request(facade_id: u32, packet_type: u8, packet: &Vec<u8>) {
    info!("hci_reset({facade_id}, {packet_type}, {packet:?})");
}

pub fn bluetooth_reset(facade_id: u32) {
    info!("hci_reset({facade_id})");
}

pub fn bluetooth_remove(facade_id: u32) {
    info!("hci_remove({facade_id})");
}

pub fn bluetooth_patch(facade_id: u32, bluetooth: &Bluetooth) {
    info!("hci_patch({facade_id}, {bluetooth:?})");
}

pub fn bluetooth_get(facade_id: u32) -> Bluetooth {
    info!("hci_get({facade_id})");
    Bluetooth::new()
}

// Returns facade_id
pub fn bluetooth_add(device_id: u32, _address: &str) -> u32 {
    info!("hci_add({device_id})");
    let mut resource = IDS.write().unwrap();
    let facade_id = resource.current_id;
    resource.current_id += 1;
    facade_id
}

/// Starts the Bluetooth service.
pub fn bluetooth_start(_instance_num: u16) {
    info!("bluetooth service started");
}

/// Stops the Bluetooth service.
pub fn bluetooth_stop() {
    info!("bluetooth service ended");
}

/// Refresh Resource for Rust tests
pub fn refresh_resource() {
    BEACON_CHIPS.write().unwrap().clear();
    let mut id_factory = crate::bluetooth::mocked::IDS.write().unwrap();
    *id_factory = crate::bluetooth::mocked::FacadeIds::new();
}

// Avoid crossing cxx boundary in tests
pub fn bluetooth_beacon_add(
    device_id: DeviceIdentifier,
    device_name: String,
    chip_id: ChipIdentifier,
    chip_proto: &ChipCreate,
) -> Result<FacadeIdentifier, String> {
    let beacon_proto = match &chip_proto.chip {
        Some(Builtin::BleBeacon(beacon_proto)) => beacon_proto,
        _ => return Err(String::from("failed to create ble beacon: unexpected chip type")),
    };

    let beacon_chip = BeaconChip::from_proto(device_name, chip_id, beacon_proto)?;
    if BEACON_CHIPS.write().unwrap().insert(chip_id, Mutex::new(beacon_chip)).is_some() {
        return Err(format!(
            "failed to create a bluetooth beacon chip with id {chip_id}: chip id already exists.",
        ));
    }

    let mut resource = crate::bluetooth::mocked::IDS.write().unwrap();
    let facade_id = resource.current_id;
    resource.current_id += 1;

    Ok(facade_id)
}
