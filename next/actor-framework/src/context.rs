//! # Actor Framework Context
//!
//! This module defines the `Context` trait and its standard implementation, `FrameworkContext`.
//! The context allows the implementation of actors to interact with the framework environment:
//! - Time management (setting tick intervals).
//! - Asynchronous stream management.
//! - Lifecycle control (shutdown signals).
//!
//! This abstraction allows for easy testing by mocking the context, as well as
//! providing a consistent interface for actors to interact with the underlying
//! execution environment.

use crate::BoxStream;
use futures::future::BoxFuture;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio_stream::{StreamMap, StreamNotifyClose};

/// The runtime environment for an actor, providing access to time, streams, and lifecycle.
pub type DynContext<Id> = dyn Context<Id> + Send;

/// The runtime environment for an actor. The ID type must be `Send + Copy + 'static`.
pub trait Context<Id>: Send + 'static {
    /// Schedule a message to be sent to the actor after a delay.oop.
    fn set_interval(&mut self, duration: Duration);

    /// Adds a new stream to be managed by the actor.
    fn add_stream(&mut self, id: Id, stream: BoxStream);

    /// Removes a managed stream by its ID.
    fn remove_stream(&mut self, id: Id);

    /// Spawns a background task to be managed by the runtime.
    ///
    /// The task is identified by `id`. When it completes, the actor's `on_task_closed` hook will be called
    /// with the value returned by the task (which must be its `id`).
    fn spawn(&mut self, id: Id, task: BoxFuture<'static, Id>);

    /// Aborts a background task by its ID.
    fn abort(&mut self, id: Id);

    /// Signals the actor to stop processing messages and exit its run loop.
    fn shutdown(&mut self);
}

pub(crate) struct FrameworkContext<Id> {
    pub(crate) interval: tokio::time::Interval,
    pub(crate) streams: StreamMap<Id, StreamNotifyClose<BoxStream>>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
    pub(crate) tasks: tokio::task::JoinSet<Id>,
    pub(crate) task_handles: std::collections::HashMap<Id, tokio::task::AbortHandle>,
}

impl<Id> FrameworkContext<Id>
where
    Id: std::hash::Hash + Eq + Copy + Send + 'static,
{
    /// Creates a new FrameworkContext with default settings.
    /// Returns the FrameworkContext and a shutdown receiver.
    pub(crate) fn new() -> (Self, oneshot::Receiver<()>) {
        // Default interval is effectively "never" (10 years)
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(365 * 10 * 24 * 60 * 60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        (
            Self {
                interval,
                streams: StreamMap::new(),
                shutdown_tx: Some(shutdown_tx),
                tasks: tokio::task::JoinSet::new(),
                task_handles: std::collections::HashMap::new(),
            },
            shutdown_rx,
        )
    }
}

impl<Id> Context<Id> for FrameworkContext<Id>
where
    Id: std::hash::Hash + Eq + Copy + Send + 'static,
{
    fn set_interval(&mut self, duration: Duration) {
        let mut interval = tokio::time::interval(duration);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        self.interval = interval;
    }

    fn add_stream(&mut self, id: Id, stream: BoxStream) {
        self.streams.insert(id, StreamNotifyClose::new(stream));
    }

    fn remove_stream(&mut self, id: Id) {
        self.streams.remove(&id);
    }

    fn spawn(&mut self, id: Id, task: BoxFuture<'static, Id>) {
        let handle = self.tasks.spawn(async move { task.await });
        self.task_handles.insert(id, handle);
    }

    fn abort(&mut self, id: Id) {
        if let Some(handle) = self.task_handles.remove(&id) {
            handle.abort();
        }
    }

    fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.tasks.abort_all();
        self.task_handles.clear();
    }
}
