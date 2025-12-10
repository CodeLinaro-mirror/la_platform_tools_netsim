// Copyright 2025 The Android Open Source Project

//! # Link Actor
//!
//! This crate implements the Link Actor for Netsim.
//! It manages the lifecycle of links between chips and handles link-related actions.
//!
//! ## Design
//!
//! The Link Actor follows the `ResourceActor` pattern from the `actor-framework`.
//! It maintains a collection of `LinkEntity` objects, each representing a connection
//! between two chips.
//!
//! ## Key Components
//!
//! - `LinkEntity`: Represents a single link.
//! - `LinkContext`: Shared state, including chip mappings and lookup tables.
//! - `handlers`: Implements the logic for processing `LinkAction`s.

pub mod actor_impl;
pub mod entity;
pub mod error;
pub mod handlers;
mod tests;

pub use entity::LinkEntity;
pub use error::LinkError;

use actor_framework::{ResourceActor, ResourceClient};

/// Creates a new LinkActor and its client.
pub fn new() -> (ResourceActor<LinkEntity>, ResourceClient<LinkEntity>) {
    ResourceActor::new(10) // Channel capacity
}
