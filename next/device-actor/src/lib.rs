// Copyright (C) 2025 The Android Open Source Project

//! # Device Actor
//!
//! This module implements the Device resource actor.
//!
//! ## Structure
//!
// Link to external crate types properly or use full paths
//! - [`service`] - [`ActorService`](actor_framework::ActorService) implementation for [`DeviceEntity`]
//! - [`error`] - [`DeviceError`] type for type-safe error handling
//! - [`handlers`] - internal request handlers
//! - [`new()`] - Factory function that creates the actor and client
//!
//! ## Custom Actions
//!
//! The Device actor showcases the Action pattern for domain-specific operations.
//!
//! ## Usage
//!
//! ```text
//! // use netsim_model::device::api::DeviceCreate;
//!
//! // #[tokio::main]
//! // async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! //     // Create actor and client
//! //     // let (actor, generic_client) = device_actor::new();
//! //     // ...
//! //     // Ok(())
//! // }
//! ```
//!
//! ## Key Features
//!
//! - **Custom actions**: Supported via `DeviceAction` (from `device_api`)
//! - **Type-safe results**: Actions return strongly-typed `DeviceActionResult`

mod actor;
mod actor_impl;
mod error;
mod handlers;
mod service;

pub use actor::DeviceActor;
pub use error::*;

use actor_framework::{ResourceActor, ResourceClient};
pub use service::DeviceEntity;

/// Creates a new Device actor and its client.
pub fn new() -> (ResourceActor<DeviceEntity>, ResourceClient<DeviceEntity>) {
    ResourceActor::new(32)
}
