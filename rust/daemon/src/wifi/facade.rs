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

use crate::ffi::ffi_wifi;
use netsim_proto::model::chip::Radio;
use protobuf::Message;

pub fn handle_wifi_request(facade_id: u32, packet: &Vec<u8>) {
    ffi_wifi::handle_wifi_request(facade_id, packet);
}

pub fn wifi_reset(facade_id: u32) {
    ffi_wifi::wifi_reset(facade_id);
}

pub fn wifi_remove(facade_id: u32) {
    ffi_wifi::wifi_remove(facade_id);
}

pub fn wifi_patch(facade_id: u32, radio: &Radio) {
    let radio_bytes = radio.write_to_bytes().unwrap();
    ffi_wifi::wifi_patch_cxx(facade_id, &radio_bytes);
}

pub fn wifi_get(facade_id: u32) -> Radio {
    let radio_bytes = ffi_wifi::wifi_get_cxx(facade_id);
    Radio::parse_from_bytes(&radio_bytes).unwrap()
}

// Returns facade_id
pub fn wifi_add(device_id: u32) -> u32 {
    ffi_wifi::wifi_add(device_id)
}

/// Starts the WiFi service.
pub fn wifi_start() {
    ffi_wifi::wifi_start();
}

/// Stops the WiFi service.
pub fn wifi_stop() {
    ffi_wifi::wifi_stop();
}
