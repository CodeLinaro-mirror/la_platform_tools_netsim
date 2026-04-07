// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/time.rs

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
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
    base: Instant,
    offset_nanos: Arc<AtomicU64>,
}

impl Default for MockClock {
    fn default() -> Self {
        Self {
            // Start the mock clock at a fixed, known time.
            base: Instant::now(),
            offset_nanos: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl MockClock {
    pub fn advance(&self, duration: Duration) {
        self.offset_nanos.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed))
    }

    fn clone_box(&self) -> Arc<dyn Clock> {
        Arc::new(self.clone())
    }
}
