// Copyright 2026 The Android Open Source Project

mod actions;
mod client;
mod error;
mod lifecycle;
mod ranging;
mod ranging_estimator;
mod service;
mod uwb_actor;

pub use actions::{UwbAction, UwbActionResult};
use actor_framework::ResourceActor;
pub use client::UwbClient;
pub use error::UwbError;
pub use uwb_actor::UwbActor;

/// Creates a new UWB actor runner and its client.
pub fn new() -> (ResourceActor<UwbActor>, UwbClient) {
    let (actor, resource_client) = ResourceActor::new(32);
    (actor, UwbClient(resource_client))
}
