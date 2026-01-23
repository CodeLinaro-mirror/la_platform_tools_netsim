#![allow(dead_code)]
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

// [cfg(test)] gets compiled during local Rust unit tests
// [cfg(not(test))] avoids getting compiled during local Rust unit tests

pub(crate) mod error;
#[cfg_attr(feature = "cuttlefish", path = "hostapd_cf.rs")]
pub(crate) mod hostapd;
#[cfg_attr(feature = "cuttlefish", path = "libslirp_cf.rs")]
pub(crate) mod libslirp;
#[cfg(not(feature = "cuttlefish"))]
pub(crate) mod mdns_forwarder;
pub(crate) mod medium;
pub(crate) mod radiotap; // touch
pub(crate) mod server;
pub(crate) mod stats;

pub use server::Server;

// TODO: Replace with global runtime
use std::sync::OnceLock;
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
use tokio::runtime::{Handle, Runtime};
pub fn get_runtime() -> Handle {
    RUNTIME.get_or_init(|| Runtime::new().unwrap()).handle().clone()
}

// TODO: Replace with DebugArgs from args
#[derive(Default)]
struct DebugArgs {
    /// Disable all packet processing and forwarding.
    pub debug_no_traffic: bool,

    /// Disable forwarding packets to the external network.
    pub debug_no_network: bool,

    /// Disable packet forwarding between simulated devices.
    pub debug_no_wmedium: bool,

    /// Disable mDNS traffic between simulated devices.
    pub debug_no_mdns_wmedium: bool,

    /// Disable mDNS traffic from guest to host.
    pub debug_no_guest_to_host_mdns: bool,
}
