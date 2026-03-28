// Copyright 2025 The Android Open Source Project

pub mod client;
pub mod error;
pub mod ftm;
pub mod gateway;
pub mod lifecycle;
pub mod mdns_forwarder;
pub mod medium;
pub mod service;
pub mod slirp_gateway;
pub mod stats;
#[cfg(target_os = "linux")]
pub mod tap_gateway;
pub mod wifi_actor;

use actor_framework::ResourceActor;
pub use client::WifiClient;
pub use error::WifiError;
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
