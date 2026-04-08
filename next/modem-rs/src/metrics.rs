// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/metrics.rs

use std::sync::atomic::{AtomicU64, Ordering};

/// A thread-safe, atomically-incremented set of counters for modem events.
#[derive(Debug, Default)]
pub struct Metrics {
    pub at_commands_received: AtomicU64,
    pub sms_sent: AtomicU64,
    pub calls_initiated: AtomicU64,
    pub calls_answered: AtomicU64,
    pub calls_hung_up: AtomicU64,
}

/// A snapshot of the modem metrics at a specific point in time.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct MetricsSnapshot {
    pub at_commands_received: u64,
    pub sms_sent: u64,
    pub calls_initiated: u64,
    pub calls_answered: u64,
    pub calls_hung_up: u64,
}

impl Metrics {
    /// Creates a snapshot of the current metric counts.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            at_commands_received: self.at_commands_received.load(Ordering::Relaxed),
            sms_sent: self.sms_sent.load(Ordering::Relaxed),
            calls_initiated: self.calls_initiated.load(Ordering::Relaxed),
            calls_answered: self.calls_answered.load(Ordering::Relaxed),
            calls_hung_up: self.calls_hung_up.load(Ordering::Relaxed),
        }
    }
}
