// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// An abstraction for time to allow for mocking in tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

impl<C: Clock + ?Sized> Clock for Box<C> {
    fn now(&self) -> Instant {
        (**self).now()
    }
}

impl<C: Clock> Clock for Arc<Mutex<C>> {
    fn now(&self) -> Instant {
        self.lock().unwrap().now()
    }
}

/// The production clock, which uses the system's real time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A mock clock for use in tests, allowing for manual time control.
#[derive(Debug, Clone)]
pub struct MockClock {
    now: Instant,
}

impl MockClock {
    pub fn new() -> Self {
        Self { now: Instant::now() }
    }

    pub fn advance(&mut self, duration: Duration) {
        self.now += duration;
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.now
    }
}

#[cfg(test)]
#[path = "tests/clock_tests.rs"]
mod tests;
