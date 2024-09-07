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
/// hostapd.conf file is generated to discovery directory.
///
use bytes::Bytes;
use log::warn;
use netsim_common::util::ini_file::IniFile;
use netsim_common::util::os_utils::get_discovery_directory;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};

pub struct Hostapd {
    running: Arc<AtomicBool>,
    verbose: bool,
    config: HashMap<String, String>,
}

impl Hostapd {
    pub fn new(_tx_bytes: mpsc::Sender<Bytes>, verbose: bool) -> Hostapd {
        let config_data = vec![
            ("ssid", "AndroidWifi-rs"),
            ("interface", "wlan1"),
            ("driver", "virtio_wifi"),
            ("bssid", "00:13:10:95:fe:0b"),
            ("country_code", "US"),
            ("hw_mode", "g"),
            ("channel", "8"),
            ("beacon_int", "1000"),
            ("dtim_period", "2"),
            ("max_num_sta", "255"),
            ("rts_threshold", "2347"),
            ("fragm_threshold", "2346"),
            ("macaddr_acl", "0"),
            ("auth_algs", "3"),
            ("ignore_broadcast_ssid", "0"),
            ("wmm_enabled", "0"),
            ("ieee80211n", "1"),
            ("eapol_key_index_workaround", "0"),
        ];
        let mut config: HashMap<String, String> = HashMap::new();
        config.extend(config_data.into_iter().map(|(k, v)| (k.to_string(), v.to_string())));
        Hostapd { running: Arc::new(AtomicBool::new(false)), verbose: verbose, config: config }
    }

    pub fn init(&mut self) -> bool {
        // TODO: create socket similar to HostapdController.cpp
        return true;
    }

    pub fn run(&self) -> bool {
        // Check if already running and exit
        if self.running.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_err()
        {
            return false;
        }
        let config_file = match self.gen_config_file() {
            Ok(filepath) => filepath,
            Err(e) => {
                warn!("Error generating hostapd conf: {:?}", e);
                return false;
            }
        };
        let verbose = self.verbose;
        let running = self.running.clone();
        if let Err(e) = thread::Builder::new().name("hostapd".to_string()).spawn({
            move || {
                let mut args = vec![CString::new("hostapd").unwrap()];
                if verbose {
                    args.push(CString::new("-dd").unwrap())
                }
                args.push(
                    CString::new(config_file.clone()).expect(&format!(
                        "CString::new error on config file path: {}",
                        config_file
                    )),
                );
                let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
                // Null-terminate the array
                argv.push(std::ptr::null());
                // Number of arguments (excluding the null terminator)
                let _argc = argv.len() as c_int - 1;
                // TODO: Invoke run_hostapd_main with argc and argv once implementation is ready
                running.store(false, Ordering::Relaxed);
            }
        }) {
            warn!("hostapd thread error: {:?}", e);
            return false;
        }
        true
    }

    pub fn set_ssid(&self, _ssid: String, _password: String) -> bool {
        todo!();
    }

    pub fn get_ssid(&self) -> Option<String> {
        self.config.get("ssid").cloned()
    }

    pub fn terminate(self) {
        todo!();
    }

    pub fn input(&self, _bytes: Bytes) {
        todo!();
    }

    // Generate hostapd.conf with fields in config
    fn gen_config_file(&self) -> anyhow::Result<String> {
        // Get the filepath of hostapd.conf under discovery directory
        let filepath = get_discovery_directory().join("hostapd.conf");
        let mut conf_file = IniFile::new(filepath.clone());

        // Iterate over the map to insert each field
        for (key, value) in &self.config {
            conf_file.insert(key, value);
        }
        if let Err(err) = conf_file.write() {
            warn!("{err:?}");
        }
        Ok(filepath.to_string_lossy().into_owned())
    }

    // TODO: Add additional functions from HostapdController.cpp as needed
}
