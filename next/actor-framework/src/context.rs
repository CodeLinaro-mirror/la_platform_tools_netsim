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

/// The runtime environment for an actor, providing access to capabilities.
pub trait Context: Send {
    /// Sets the interval for the actor's tick loop.
    fn set_interval(&mut self, duration: Duration);

    /// Adds a new stream to be managed by the actor.
    fn add_stream(&mut self, id: usize, stream: BoxStream);

    /// Adds a background task to be managed by the runtime.
    ///
    /// The task is identified by `id`. When it completes, the actor's `on_task_closed` hook will be called.
    fn add_task(&mut self, id: usize, task: BoxFuture<'static, ()>);

    /// Signals the actor to stop processing messages and exit its run loop.
    fn shutdown(&mut self);
}

pub struct FrameworkContext {
    pub(crate) interval: tokio::time::Interval,
    pub(crate) streams: StreamMap<usize, StreamNotifyClose<BoxStream>>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
    pub(crate) tasks: tokio::task::JoinSet<usize>,
}

impl FrameworkContext {
    /// Creates a new FrameworkContext with default settings.
    /// Returns the FrameworkContext and a shutdown receiver.
    pub fn new() -> (Self, oneshot::Receiver<()>) {
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
            },
            shutdown_rx,
        )
    }
}

impl Context for FrameworkContext {
    fn set_interval(&mut self, duration: Duration) {
        let mut interval = tokio::time::interval(duration);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        self.interval = interval;
    }

    fn add_stream(&mut self, id: usize, stream: BoxStream) {
        self.streams.insert(id, StreamNotifyClose::new(stream));
    }

    fn add_task(
        &mut self,
        id: usize,
        task: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
    ) {
        self.tasks.spawn(async move {
            task.await;
            id
        });
    }

    fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.tasks.abort_all();
    }
}
