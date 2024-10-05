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

/// Hostapd Interface for Network Simulation
use bytes::Bytes;
#[cfg(not(feature = "cuttlefish"))]
pub use hostapd_rs::hostapd::Hostapd;
#[cfg(not(feature = "cuttlefish"))]
use netsim_common::util::os_utils::get_discovery_directory;
use netsim_proto::config::HostapdOptions as ProtoHostapdOptions;
use std::sync::mpsc;

// Provides a stub implementation while the hostapd-rs crate is not integrated into the aosp-main.
#[cfg(feature = "cuttlefish")]
pub struct Hostapd {}
#[cfg(feature = "cuttlefish")]
impl Hostapd {
    pub fn input(&self, _bytes: Bytes) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "cuttlefish"))]
pub fn hostapd_run(_opt: ProtoHostapdOptions, tx: mpsc::Sender<Bytes>) -> anyhow::Result<Hostapd> {
    // Create hostapd.conf under discovery directory
    let config_path = get_discovery_directory().join("hostapd.conf");
    let mut hostapd = Hostapd::new(tx, true, config_path);
    hostapd.run();
    Ok(hostapd)
}

#[cfg(feature = "cuttlefish")]
pub fn hostapd_run(_opt: ProtoHostapdOptions, _tx: mpsc::Sender<Bytes>) -> anyhow::Result<Hostapd> {
    Ok(Hostapd {})
}
