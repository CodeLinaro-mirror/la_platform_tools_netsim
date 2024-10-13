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

/// Test whether hostapd starts and terminates successfully
#[test]
fn test_hostapd_run_and_terminate() {
    let (tx, _rx): (mpsc::Sender<Bytes>, mpsc::Receiver<Bytes>) = mpsc::channel();
    let config_path = env::temp_dir().join("hostapd.conf");
    let mut hostapd = Hostapd::new(tx, true, config_path);
    hostapd.run();
    assert!(hostapd.is_running());
    hostapd.terminate();
    // Check that hostapd terminates within 30s
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
