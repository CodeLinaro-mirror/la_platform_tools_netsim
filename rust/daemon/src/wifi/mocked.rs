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

use lazy_static::lazy_static;
use log::info;
use netsim_proto::model::chip::Radio;
use std::sync::RwLock;

lazy_static! {
    static ref IDS: RwLock<FacadeIds> = RwLock::new(FacadeIds::new());
}

struct FacadeIds {
    current_id: u32,
}

impl FacadeIds {
    fn new() -> Self {
        FacadeIds { current_id: 0 }
    }
}

pub fn handle_wifi_request(facade_id: u32, packet: &Vec<u8>) {
    info!("handle_wifi_request({facade_id}, {packet:?})");
}

pub fn wifi_reset(facade_id: u32) {
    info!("wifi_reset({facade_id})");
}

pub fn wifi_remove(facade_id: u32) {
    info!("wifi_remove({facade_id})");
}

pub fn wifi_patch(facade_id: u32, radio: &Radio) {
    info!("wifi_patch({facade_id}, {radio:?})");
}

pub fn wifi_get(facade_id: u32) -> Radio {
    info!("wifi_get({facade_id})");
    Radio::new()
}

// Returns facade_id
pub fn wifi_add(device_id: u32) -> u32 {
    info!("wifi_add({device_id})");
    let mut resource = IDS.write().unwrap();
    let facade_id = resource.current_id;
    resource.current_id += 1;
    facade_id
}

/// Starts the WiFi service.
pub fn wifi_start() {
    info!("wifi service started");
}

/// Stops the WiFi service.
pub fn wifi_stop() {
    info!("wifi service stopped");
}

/// Refresh Resource for Rust tests
pub fn refresh_resource() {
    let mut resource = IDS.write().unwrap();
    resource.current_id = 0;
}
