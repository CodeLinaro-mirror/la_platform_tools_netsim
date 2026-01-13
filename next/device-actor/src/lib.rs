// Copyright 2025 The Android Open Source Project

//! Device Actor
//!
//! This crate provides the `DeviceActor` which manages the lifecycle of devices
//! and their associated chips.

mod device_actor;
mod error;
mod lifecycle;
mod service;
mod utils;

pub mod client;
pub use actor_framework::{ResourceActor, ResourceClient};
pub use client::DeviceClient;
pub use device_actor::DeviceActor;
pub use error::DeviceError;

/// Creates a new Device actor runner and its client.
pub fn new() -> (ResourceActor<DeviceActor>, DeviceClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, DeviceClient::new(Box::new(client)))
}
