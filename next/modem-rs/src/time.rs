// src/time.rs

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// A trait that abstracts away the concept of "now" for testability.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn clone_box(&self) -> Arc<dyn Clock>;
}

/// The real clock implementation for production code.
#[derive(Clone, Debug)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn clone_box(&self) -> Arc<dyn Clock> {
        Arc::new(self.clone())
    }
}

/// The mock clock implementation for testing.
#[derive(Clone, Debug)]
pub struct MockClock {
    current_time: Arc<Mutex<Instant>>,
}

impl MockClock {
    pub fn new() -> Self {
        Self {
            // Start the mock clock at a fixed, known time.
            current_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn advance(&self, duration: Duration) {
        let mut time = self.current_time.lock().unwrap();
        *time += duration;
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        *self.current_time.lock().unwrap()
    }

    fn clone_box(&self) -> Arc<dyn Clock> {
        Arc::new(self.clone())
    }
}
