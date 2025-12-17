//! # ActorLifecycle Trait
//!
//! The `ActorLifecycle` trait defines the contract for an actor's lifecycle hooks.
//! This allows the Actor Implementation to perform operations outside of ActorService handlers.

use crate::DynContext;
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
pub trait ActorLifecycle<Id>: Send + 'static
where
    Id: Send + Copy + From<u32> + Into<u32> + 'static,
{
    /// The error type returned by context methods.
    type Error;

    /// Called when the actor starts, before processing any messages.
    ///
    /// Use this hook to:
    /// - Schedule initial timers.
    /// - Register initial streams.
    async fn on_start(&mut self, _ctx: &mut DynContext<Id>) {}

    /// Called on every tick of the actor's interval.
    async fn on_tick(&mut self, _ctx: &mut DynContext<Id>) {}

    /// Called when a stream produces a message.
    async fn on_stream(&mut self, _id: Id, _message: StreamMessage, _ctx: &mut DynContext<Id>) {}

    /// Hook called when a background task managed by `spawn` completes.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the task that completed.
    async fn on_task_closed(&mut self, id: Id, _ctx: &mut DynContext<Id>) {
        log::debug!("Task closed: {}", id.into());
    }

    /// Called when a registered stream closes.
    async fn on_stream_closed(&mut self, id: Id, _ctx: &mut DynContext<Id>) {
        log::debug!("Stream closed: {}", id.into());
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
impl ActorLifecycle<u32> for () {
    type Error = EmptyError;
}
