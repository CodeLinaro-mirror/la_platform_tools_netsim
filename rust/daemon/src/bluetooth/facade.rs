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

use crate::ffi::ffi_bluetooth;
use ::protobuf::MessageField;
use cxx::let_cxx_string;
use netsim_proto::config::Bluetooth as BluetoothConfig;
use netsim_proto::model::chip::Bluetooth;
use protobuf::Message;

pub fn handle_bluetooth_request(facade_id: u32, packet_type: u8, packet: &Vec<u8>) {
    ffi_bluetooth::handle_bt_request(facade_id, packet_type, packet);
}

pub fn bluetooth_reset(facade_id: u32) {
    ffi_bluetooth::bluetooth_reset(facade_id);
}

pub fn bluetooth_remove(facade_id: u32) {
    ffi_bluetooth::bluetooth_remove(facade_id);
}

pub fn bluetooth_patch(facade_id: u32, bluetooth: &Bluetooth) {
    let bluetooth_bytes = bluetooth.write_to_bytes().unwrap();
    ffi_bluetooth::bluetooth_patch_cxx(facade_id, &bluetooth_bytes);
}

pub fn bluetooth_get(facade_id: u32) -> Bluetooth {
    let bluetooth_bytes = ffi_bluetooth::bluetooth_get_cxx(facade_id);
    Bluetooth::parse_from_bytes(&bluetooth_bytes).unwrap()
}

// Returns facade_id
pub fn bluetooth_add(device_id: u32, address: &str) -> u32 {
    let_cxx_string!(cxx_address = address);
    ffi_bluetooth::bluetooth_add(device_id, &cxx_address)
}

/// Starts the Bluetooth service.
pub fn bluetooth_start(config: &MessageField<BluetoothConfig>, instance_num: u16) {
    let proto_bytes = config.as_ref().unwrap_or_default().write_to_bytes().unwrap();
    ffi_bluetooth::bluetooth_start(&proto_bytes, instance_num);
}

/// Stops the Bluetooth service.
pub fn bluetooth_stop() {
    ffi_bluetooth::bluetooth_stop();
}
