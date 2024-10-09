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
use std::sync::mpsc;

/// Test whether run() starts the hostapd thread
#[test]
fn test_hostapd_run() {
    let (tx, _rx): (mpsc::Sender<Bytes>, mpsc::Receiver<Bytes>) = mpsc::channel();
    let config_path = std::env::temp_dir().join("hostapd.conf");
    let mut hostapd = Hostapd::new(tx, true, config_path);
    hostapd.run();
    assert!(hostapd.is_running());
}
