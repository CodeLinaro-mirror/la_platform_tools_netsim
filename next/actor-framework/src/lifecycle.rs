//! # ActorLifecycle Trait
//!
//! The `ActorLifecycle` trait defines the contract for an actor's lifecycle
//! hooks. This allows the Actor Implementation to perform operations outside of
//! ActorService handlers.

use bytes::Bytes;

use crate::{ActorService, DynContext};

pub type StreamMessage = Bytes;

/// Defines the lifecycle hooks for an actor's shared state (Context).
///
/// This trait corresponds to the **State Holder** layer
/// ("When I am invoked"). It allows the actor's context to respond to
/// framework-level events like startup, shutdown, and timer ticks.
///
/// This allows the Actor Implementation to perform operations outside of
/// ActorService handlers.
pub trait ActorLifecycle: ActorService {
    /// Called when the actor starts, before processing any messages.
    ///
    /// Use this hook to:
    /// - Schedule initial timers.
    /// - Register initial streams.
    fn on_start(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = ()> + Send {
        futures::future::ready(())
    }

    /// Called on every tick of the actor's interval.
    fn on_tick(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = ()> + Send {
        futures::future::ready(())
    }

    /// Called when a stream produces a message.
    fn on_stream(
        &mut self,
        _id: Self::Id,
        _message: StreamMessage,
        _ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = ()> + Send {
        futures::future::ready(())
    }

    /// Hook called when a background task managed by `spawn` completes.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the task that completed.
    fn on_task_closed(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = ()> + Send {
        log::debug!("Task closed: {}", id.into());
        futures::future::ready(())
    }

    /// Called when a registered stream closes.
    fn on_stream_closed(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = ()> + Send {
        log::debug!("Stream closed: {}", id.into());
        futures::future::ready(())
    }

    /// Called when the actor receives a shutdown signal.
    fn on_shutdown(&mut self) -> impl std::future::Future<Output = ()> + Send {
        futures::future::ready(())
    }
}
