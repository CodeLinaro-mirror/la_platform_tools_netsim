// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod client;
pub(crate) mod error;
pub(crate) mod ethernet_actor;
pub(crate) mod lifecycle;

use actor_framework::ResourceActor;
pub use client::EthernetClient;
pub use error::EthernetError;
pub use ethernet_actor::{EthernetActor, EthernetReq, EthernetResponse};

/// Creates a new EthernetActor runner and client.
pub fn new() -> (ResourceActor<EthernetActor>, EthernetClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, EthernetClient::new(client))
}
