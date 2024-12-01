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
use netsim_proto::config::HostapdOptions as ProtoHostapdOptions;
use std::sync::mpsc;

// Provides a stub implementation while the hostapd-rs crate is not integrated into the aosp-main.
pub struct Hostapd {}
impl Hostapd {
    pub fn input(&self, _bytes: Bytes) -> anyhow::Result<()> {
        Ok(())
    }
}

pub fn hostapd_run(_opt: ProtoHostapdOptions, _tx: mpsc::Sender<Bytes>) -> anyhow::Result<Hostapd> {
    Ok(Hostapd {})
}
