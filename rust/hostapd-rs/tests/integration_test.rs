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

use bytes::Bytes;
use hostapd_rs::hostapd::Hostapd;
use std::{
    env,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Helper function to initialize Hostapd for test
fn init_test_hostapd() -> Hostapd {
    let (tx, _rx): (mpsc::Sender<Bytes>, mpsc::Receiver<Bytes>) = mpsc::channel();
    let config_path = env::temp_dir().join("hostapd.conf");
    Hostapd::new(tx, true, config_path)
}

/// Hostpad integration test
///
/// A single test is used here to avoid conflicts when multiple hostapd runs in parallel
/// TODO: Split up tests once serial_test crate is available
#[test]
fn test_hostapd() {
    test_start_and_terminate();
    test_set_ssid();
}

/// Test Hostapd starts and terminates successfully
fn test_start_and_terminate() {
    // Test hostapd starts successfully
    let mut hostapd = init_test_hostapd();
    hostapd.run();
    assert!(hostapd.is_running());

    // Test hostapd terminates successfully
    hostapd.terminate();
    let max_wait_time = Duration::from_secs(30);
    let start_time = Instant::now();
    while start_time.elapsed() < max_wait_time {
        if !hostapd.is_running() {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    assert!(!hostapd.is_running());
}

/// Test various ways to configure Hostapd SSID and password
fn test_set_ssid() {
    let mut hostapd = init_test_hostapd();
    hostapd.run();
    assert!(hostapd.is_running());

    // Check default ssid is set
    assert_eq!(hostapd.get_ssid(), "AndroidWifi");
    let mut test_ssid = String::new();
    let mut test_password = String::new();

    // Verify set_ssid fails if SSID is empty
    assert!(hostapd.set_ssid(test_ssid.clone(), test_password.clone()).is_err());

    // Verify set_ssid succeeds if SSID is not empty
    test_ssid = "TestSsid".to_string();
    assert!(hostapd.set_ssid(test_ssid.clone(), test_password.clone()).is_ok());
    // TODO: Enhance test to verify hostapd response packet SSID

    // Verify ssid was set successfully
    assert_eq!(hostapd.get_ssid(), test_ssid.clone());

    // Verify setting same ssid again succeeds
    assert!(hostapd.set_ssid(test_ssid.clone(), test_password.clone()).is_ok());

    // Verify set_ssid fails if password is not empty
    // TODO: Update once password support is implemented
    test_password = "TestPassword".to_string();
    assert!(hostapd.set_ssid(test_ssid.clone(), test_password.clone()).is_err());
}
