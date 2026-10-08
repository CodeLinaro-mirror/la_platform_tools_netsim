// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/time.rs

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use jiff::Zoned;
use parking_lot::RwLock;

/// A trait that abstracts away the concept of "now" for testability.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn now_zoned(&self) -> Zoned;
    fn clone_box(&self) -> Arc<dyn Clock>;
}

/// The real clock implementation for production code.
#[derive(Clone, Debug)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn now_zoned(&self) -> Zoned {
        Zoned::now()
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
    zoned: Arc<RwLock<(Zoned, u64)>>,
}

impl Default for MockClock {
    fn default() -> Self {
        Self {
            base: Instant::now(),
            offset_nanos: Arc::new(AtomicU64::new(0)),
            zoned: Arc::new(RwLock::new((Zoned::now(), 0))),
        }
    }
}

impl MockClock {
    pub fn advance(&self, duration: Duration) {
        self.offset_nanos.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn set_zoned(&self, zoned: Zoned) {
        let current_offset = self.offset_nanos.load(Ordering::Relaxed);
        *self.zoned.write() = (zoned, current_offset);
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_nanos(self.offset_nanos.load(Ordering::Relaxed))
    }

    fn now_zoned(&self) -> Zoned {
        let current_offset = self.offset_nanos.load(Ordering::Relaxed);
        let (base, base_offset) = self.zoned.read().clone();
        let delta = Duration::from_nanos(current_offset.saturating_sub(base_offset));
        base.checked_add(delta).unwrap_or(base)
    }

    fn clone_box(&self) -> Arc<dyn Clock> {
        Arc::new(self.clone())
    }
}
