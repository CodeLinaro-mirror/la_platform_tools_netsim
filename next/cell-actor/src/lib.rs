// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod actions;
mod cell_actor;
mod error;
mod lifecycle;
mod service;

mod client;
// Re-export core types
pub use actions::{CellAction, CellActionResult};
pub use actor_framework::ResourceActor;
pub use cell_actor::CellActor;
/// A type alias for the client used to interact with the Cell actor.
pub use client::CellClient;
pub use error::CellError;

/// Creates a new Cell actor framework instance and client.
///
/// Returns:
/// - `ResourceActor`: The actor runner that drives the service.
/// - `CellClient`: The client for sending requests to the actor.
///
/// The caller is responsible for creating the `CellActor` service and spawning
/// the runner: `tokio::spawn(actor.run(service));`
pub fn new() -> (ResourceActor<CellActor>, CellClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, CellClient(client))
}
