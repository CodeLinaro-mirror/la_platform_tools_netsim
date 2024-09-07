// Copyright 2024 Google LLC
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

///
/// This crate is a wrapper for hostapd C library.
///
/// Hostapd process is managed by a separate thread.
///
/// hostapd.conf file is generated and to discovery directory.
///
use bytes::Bytes;
use std::sync::mpsc;
// API to Hostapd

pub struct Hostapd {}

impl Hostapd {
    pub fn new(_tx_bytes: mpsc::Sender<Bytes>, _verbose: bool) -> Hostapd {
        Hostapd {}
    }

    pub fn init(&mut self) -> bool {
        todo!();
    }

    pub fn run(&self) -> bool {
        todo!();
    }

    pub fn set_ssid(&self, _ssid: String, _password: String) -> bool {
        todo!();
    }

    pub fn get_ssid(&self) -> String {
        todo!();
    }

    pub fn terminate(self) {
        todo!();
    }

    pub fn input(&self, _bytes: Bytes) {
        todo!();
    }

    fn gen_config_file(&self) -> String {
        todo!();
    }

    // TODO: Add additional functions from HostapdController.cpp as needed
}
