use super::Runtime;
use crate::BoxStream;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio_stream::{StreamMap, StreamNotifyClose};

pub struct StandardRuntime {
    pub(crate) interval: tokio::time::Interval,
    pub(crate) streams: StreamMap<usize, StreamNotifyClose<BoxStream>>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
}

impl StandardRuntime {
    /// Creates a new StandardRuntime with default settings.
    /// Returns the StandardRuntime and a shutdown receiver.
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        // Default interval is effectively "never" (10 years)
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(365 * 10 * 24 * 60 * 60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        (Self { interval, streams: StreamMap::new(), shutdown_tx: Some(shutdown_tx) }, shutdown_rx)
    }
}

impl Runtime for StandardRuntime {
    fn set_interval(&mut self, duration: Duration) {
        let mut interval = tokio::time::interval(duration);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        self.interval = interval;
    }

    fn add_stream(&mut self, id: usize, stream: BoxStream) {
        self.streams.insert(id, StreamNotifyClose::new(stream));
    }

    fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}
