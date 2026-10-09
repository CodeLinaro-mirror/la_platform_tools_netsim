// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod client;
mod error;
mod ethernet_actor;
mod lifecycle;

use actor_framework::ResourceActor;
pub use client::EthernetClient;
pub use ethernet_actor::EthernetActor;

/// Creates a new EthernetActor runner and client.
pub fn new() -> (ResourceActor<EthernetActor>, EthernetClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, EthernetClient::new(client))
}
