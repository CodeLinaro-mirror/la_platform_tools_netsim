//! # Actor Runtime
//!
//! This module defines the `Runtime` trait and its standard implementation, `StandardRuntime`.
//! The runtime provides actors with access to:
//! - Time management (setting tick intervals).
//! - Asynchronous stream management.
//! - Lifecycle control (shutdown signals).
//!
//! This abstraction allows for easy testing by mocking the runtime, as well as
//! providing a consistent interface for actors to interact with the underlying
//! execution environment.

mod standard;

pub(crate) use standard::StandardRuntime;

use crate::BoxStream;
use std::time::Duration;

/// The runtime environment for an actor, providing access to capabilities.
pub trait Runtime: Send {
    /// Sets the interval for the actor's tick loop.
    fn set_interval(&mut self, duration: Duration);

    /// Adds a new stream to be managed by the actor.
    fn add_stream(&mut self, id: usize, stream: BoxStream);

    /// Signals the actor to stop processing messages and exit its run loop.
    fn shutdown(&mut self);
}
