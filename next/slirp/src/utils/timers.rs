// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    time::{Duration, Instant},
};

use crate::clock::{Clock, SystemClock};

/// Manages all time-based events for a Slirp instance.
pub struct TimerManager {
    events: BinaryHeap<ScheduledEvent>,
    clock: Box<dyn Clock>,
}

/// A specific event to be executed at a scheduled time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerEvent {
    Test,
    Tcp(u64),
    TcpRetransmit(u64),
    NdpRouterAdvertisement,
}

/// An event scheduled to be executed at a specific time.
#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    pub time: Instant,
    pub event: TimerEvent,
}

impl Eq for ScheduledEvent {}
impl PartialEq for ScheduledEvent {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new(Box::new(SystemClock))
    }
}

impl TimerManager {
    pub fn new(clock: Box<dyn Clock>) -> Self {
        Self { events: BinaryHeap::new(), clock }
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }

    pub fn schedule(&mut self, delay: Duration, event: TimerEvent) {
        let time = self.clock.now() + delay;
        self.events.push(ScheduledEvent { time, event });
    }

    pub fn next_event_in(&self) -> Option<Duration> {
        self.events.peek().map(|event| {
            let now = self.clock.now();
            if event.time > now { event.time - now } else { Duration::ZERO }
        })
    }

    pub fn tick(&mut self) -> Vec<TimerEvent> {
        let now = self.clock.now();
        let mut fired_events = Vec::new();
        while let Some(event) = self.events.peek() {
            if event.time <= now {
                if let Some(event) = self.events.pop() {
                    fired_events.push(event.event);
                }
            } else {
                break;
            }
        }
        fired_events
    }

    pub fn cancel_by_event(&mut self, event_to_cancel: &TimerEvent) {
        self.events.retain(|e| &e.event != event_to_cancel);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
