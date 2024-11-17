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
use log::warn;
use netsim_packets::ieee80211::Ieee80211;
use pdl_runtime::Packet;
use std::{
    env,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Helper function to initialize Hostapd for test
fn init_test_hostapd() -> (Hostapd, mpsc::Receiver<Bytes>) {
    let (tx, rx) = mpsc::channel();
    let config_path = env::temp_dir().join("hostapd.conf");
    (Hostapd::new(tx, true, config_path), rx)
}

/// Helper function to wait for Hostapd to terminate
fn terminate_hostapd(hostapd: &Hostapd) {
    hostapd.terminate();
    let max_wait_time = Duration::from_secs(30);
    let start_time = Instant::now();
    while start_time.elapsed() < max_wait_time {
        if !hostapd.is_running() {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    warn!("Hostapd failed to terminate successfully within 30s");
}

/// Hostpad integration test
///
/// A single test is used here to avoid conflicts when multiple hostapd runs in parallel
/// TODO: Split up tests once serial_test crate is available
#[test]
fn test_hostapd() {
    test_start_and_terminate();
    test_receive_beacon_frame();
    test_set_ssid();
}

/// Test Hostapd starts and terminates successfully
fn test_start_and_terminate() {
    // Test hostapd starts successfully
    let (mut hostapd, _) = init_test_hostapd();
    hostapd.run();
    assert!(hostapd.is_running());

    // Test hostapd terminates successfully
    terminate_hostapd(&hostapd);
    assert!(!hostapd.is_running());
}

/// Test whether beacon frame packet is received after Hostapd starts up
fn test_receive_beacon_frame() {
    let (mut hostapd, receiver) = init_test_hostapd();
    hostapd.run();
    assert!(hostapd.is_running());

    let end_time = Instant::now() + Duration::from_secs(10);
    loop {
        // Try to receive a packet before end_time
        match receiver.recv_timeout(end_time - Instant::now()) {
            // Parse and verify received packet is beacon frame
            Ok(packet) if Ieee80211::decode_full(&packet).unwrap().is_beacon() => break,
            Ok(_) => continue,   // Received a non beacon packet. Continue
            _ => assert!(false), // Error occurred
        }
    }
    terminate_hostapd(&hostapd);
}

fn verify_beacon_frame_ssid(receiver: &mpsc::Receiver<Bytes>, ssid: &str) {
    let end_time = Instant::now() + Duration::from_secs(10);
    loop {
        // Try to receive a packet before end_time
        match receiver.recv_timeout(end_time - Instant::now()) {
            Ok(packet) => {
                if let Ok(beacon_ssid) =
                    Ieee80211::decode_full(&packet).unwrap().get_ssid_from_beacon_frame()
                {
                    if beacon_ssid == ssid {
                        break; // Found expected beacon frame
                    }
                }
                // Not expected beacon frame. Continue...
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                assert!(false, "No Beacon frame received within 10s");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                assert!(false, "Receiver disconnected while waiting for Beacon frame.");
            }
        }
    }
}

/// Test various ways to configure Hostapd SSID and password
fn test_set_ssid() {
    let (mut hostapd, receiver) = init_test_hostapd();
    hostapd.run();
    assert!(hostapd.is_running());
    // Check default ssid is set
    let default_ssid = "AndroidWifi";
    assert_eq!(hostapd.get_ssid(), default_ssid);
    // Verify hostapd sends beacon frame with default SSID
    verify_beacon_frame_ssid(&receiver, default_ssid);

    let mut test_ssid = String::new();
    let mut test_password = String::new();
    // Verify set_ssid fails if SSID is empty
    assert!(hostapd.set_ssid(&test_ssid, &test_password).is_err());

    // Verify set_ssid succeeds if SSID is not empty
    test_ssid = "TestSsid".to_string();
    assert!(hostapd.set_ssid(&test_ssid, &test_password).is_ok());
    // Verify hostapd sends new beacon frame with updated SSID
    verify_beacon_frame_ssid(&receiver, &test_ssid);

    // Verify ssid was set successfully
    assert_eq!(hostapd.get_ssid(), test_ssid);

    // Verify setting same ssid again succeeds
    assert!(hostapd.set_ssid(&test_ssid, &test_password).is_ok());

    // Verify set_ssid fails if password is not empty
    // TODO: Update once password support is implemented
    test_password = "TestPassword".to_string();
    assert!(hostapd.set_ssid(&test_ssid, &test_password).is_err());

    terminate_hostapd(&hostapd);
}
