// Copyright 2025 The Android Open Source Project

mod error;
mod lifecycle;
mod link_actor;
mod service;

#[cfg(test)]
mod tests;

use actor_framework::{ResourceActor, ResourceClient};
pub use error::LinkError;
pub use link_actor::LinkActor;

/// Creates a new Link actor and its client.
pub fn new() -> (ResourceActor<LinkActor>, ResourceClient<LinkActor>) {
    ResourceActor::new(32)
}
