//! # ActorLifecycle Trait
//!
//! The `ActorLifecycle` trait defines the contract for an actor's lifecycle hooks.
//! This allows the Actor Implementation to perform operations outside of ActorService handlers.

use crate::Context;
use async_trait::async_trait;
use bytes::Bytes;

pub type StreamMessage = Bytes;

/// Defines the lifecycle hooks for an actor's shared state (Context).
///
/// This trait corresponds to the **State Holder** layer
/// ("When I am invoked"). It allows the actor's context to respond to
/// framework-level events like startup, shutdown, and timer ticks.
///
/// This allows the Actor Implementation to perform operations outside of ActorService handlers.
#[async_trait]
pub trait ActorLifecycle: Send + Sync + 'static {
    /// The error type returned by context methods.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Called when the actor starts, before processing any messages.
    ///
    /// Use this hook to:
    /// - Schedule initial timers.
    /// - Register initial streams.
    async fn on_start(&mut self, _runtime: &mut impl Context) {}

    /// Called on every tick of the actor's interval.
    async fn on_tick(&mut self, _runtime: &mut impl Context) {}

    /// Called when a stream produces a message.
    async fn on_stream(&mut self, _id: u32, _message: StreamMessage, ctx: &mut impl Context) {}

    /// Called when a background task completes.
    ///
    /// The `id` matches the one passed to `runtime.add_task`.
    /// Returns `Ok(true)` if the entity associated with the task ID should be deleted.
    async fn on_task_closed(&mut self, id: u32) -> Result<bool, Self::Error> {
        log::debug!("Task closed: {}", id);
        Ok(false)
    }

    /// Called when a registered stream closes.
    ///
    /// Returns `Ok(true)` if the entity associated with the stream ID should be deleted.
    /// This is useful for entities that are 1:1 with a stream (e.g., a chip connected to a packet stream).
    async fn on_stream_closed(&mut self, id: u32) -> Result<bool, Self::Error> {
        log::debug!("Stream closed: {}", id);
        Ok(false)
    }

    /// Called when the actor receives a shutdown signal.
    async fn on_shutdown(&mut self) {}
}

#[derive(Debug)]
pub struct EmptyError;
impl std::fmt::Display for EmptyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmptyError")
    }
}
impl std::error::Error for EmptyError {}

#[async_trait]
impl ActorLifecycle for () {
    type Error = EmptyError;
}
