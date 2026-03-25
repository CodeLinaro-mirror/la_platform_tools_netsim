// Copyright 2026 The Android Open Source Project

pub mod actions;
pub mod client;
pub mod error;
pub mod lifecycle;
mod ranging;
mod ranging_estimator;
pub mod service;
pub mod uwb_actor;

pub use actions::{UwbAction, UwbActionResult};
pub use actor_framework::ResourceActor;
pub use client::UwbClient;
pub use error::UwbError;
pub use uwb_actor::UwbActor;

/// Creates a new UWB actor runner and its client.
pub fn new() -> (ResourceActor<UwbActor>, UwbClient) {
    let (actor, resource_client) = ResourceActor::new(32);
    (actor, UwbClient(resource_client))
}
