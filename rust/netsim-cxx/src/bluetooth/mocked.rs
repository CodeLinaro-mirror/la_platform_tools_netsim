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

use frontend_proto::model::chip::Bluetooth;
use log::info;

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
pub fn bluetooth_add(chip_id: u32) -> u32 {
    info!("hci_add({chip_id})");
    0
}

/// Starts the Bluetooth service.
pub fn bluetooth_start() {
    info!("bluetooth service started");
}

/// Stops the Bluetooth service.
pub fn bluetooth_stop() {
    info!("bluetooth service ended");
}
