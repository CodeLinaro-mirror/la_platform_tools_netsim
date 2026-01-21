// Copyright 2025 The Android Open Source Project

mod error;
mod lifecycle;
mod service;
mod slirp_actor;

mod slirp_client;

pub use error::SlirpError;
pub use slirp_actor::{SlirpActor, SlirpCreate, SlirpReq, SlirpStatus};
pub use slirp_client::SlirpClient;

use actor_framework::ResourceActor;

/// Creates a new SlirpActor and returns the runner and a client.
pub fn new() -> (ResourceActor<SlirpActor>, SlirpClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, SlirpClient::new(client))
}
