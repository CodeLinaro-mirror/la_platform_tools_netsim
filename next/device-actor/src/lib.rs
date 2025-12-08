// Copyright (C) 2025 The Android Open Source Project

//! # Device Actor
//!
//! This module implements the Device resource actor.
//!
//! ## Structure
//!
//! - [`entity`] - [`ActorEntity`](actor_framework::ActorEntity) implementation for [`DeviceEntity`]
//! - [`error`] - [`DeviceError`] type for type-safe error handling
//! - [`actions`] - [`DeviceAction`] and [`DeviceActionResult`]
//! - [`new()`] - Factory function that creates the actor and client
//!
//! ## Custom Actions
//!
//! The Device actor showcases the Action pattern for domain-specific operations.
//!
//! ## Usage
//!
//! ```rust

//! use netsim_model::device::api::DeviceCreate;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create actor and client
//!     let (actor, generic_client) = device_actor::new();
//!     // ...
//!     Ok(())
//! }
//! ```
//!
//! ## Key Features
//!
//! - **Custom actions**: Supported via [`DeviceAction`]
//! - **Type-safe results**: Actions return strongly-typed [`DeviceActionResult`]

pub mod actor_impl;
pub mod context;
pub mod entity;
pub mod error;
pub mod handlers;

pub use context::DeviceContext;
pub use error::*;

pub use crate::entity::DeviceEntity;
use actor_framework::{ResourceActor, ResourceClient};

/// Creates a new Device actor and its client.
pub fn new() -> (ResourceActor<DeviceEntity>, ResourceClient<DeviceEntity>) {
    ResourceActor::new(32)
}
