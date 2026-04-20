// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod client;
pub(crate) mod error;
pub(crate) mod ftm;
pub(crate) mod gateway;
pub(crate) mod lifecycle;
pub(crate) mod mdns_forwarder;
pub(crate) mod medium;
pub(crate) mod service;
pub(crate) mod slirp_gateway;
pub(crate) mod stats;
#[cfg(target_os = "linux")]
pub(crate) mod tap_gateway;
pub(crate) mod wifi_actor;

use actor_framework::ResourceActor;
pub use client::WifiClient;
pub use error::WifiError;
pub use ftm::handle_ftm_request;
pub use gateway::GatewayTrait;
pub use medium::{utils::create_hwsim_msg_from_frame, Medium};
pub use stats::{Clock, MockClock, SystemClock};
#[cfg(target_os = "linux")]
pub use tap_gateway::TapGateway;
pub use wifi_actor::{WifiActor, WifiReq, WifiResponse};

#[derive(Default, Debug, Clone)]
pub struct DebugArgs {
    pub debug_no_traffic: bool,
    pub debug_no_network: bool,
    pub debug_no_wmedium: bool,
    pub debug_no_mdns_wmedium: bool,
    pub debug_no_guest_to_host_mdns: bool,
}

/// Creates a new WifiActor runner and client.
pub fn new() -> (ResourceActor<WifiActor>, WifiClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, WifiClient::new(client))
}
