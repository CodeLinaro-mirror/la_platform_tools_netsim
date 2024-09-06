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

/// LibSlirp Interface for Network Simulation
use bytes::Bytes;
#[cfg(not(feature = "cuttlefish"))]
pub use libslirp_rs::libslirp::LibSlirp;
#[cfg(not(feature = "cuttlefish"))]
use libslirp_rs::libslirp_config::SlirpConfig;
use netsim_proto::config::SlirpOptions as ProtoSlirpOptions;
use std::sync::mpsc;

// Provides a stub implementation while the libslirp-rs crate is not integrated into the aosp-main.
#[cfg(feature = "cuttlefish")]
pub struct LibSlirp {}

#[cfg(not(feature = "cuttlefish"))]
pub fn slirp_run(_opt: ProtoSlirpOptions) -> anyhow::Result<(LibSlirp, mpsc::Receiver<Bytes>)> {
    // TODO: Convert ProtoSlirpOptions to SlirpConfig.
    let config = SlirpConfig { ..Default::default() };
    let (tx, rx) = mpsc::channel();
    let slirp = LibSlirp::new(config, tx);
    Ok((slirp, rx))
}

#[cfg(feature = "cuttlefish")]
pub fn slirp_run(_opt: ProtoSlirpOptions) -> anyhow::Result<(LibSlirp, mpsc::Receiver<Bytes>)> {
    let slirp = LibSlirp {};
    let (_, rx) = mpsc::channel();
    Ok((slirp, rx))
}
