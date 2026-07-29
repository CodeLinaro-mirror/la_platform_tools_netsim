// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::utils::clock::{Clock, MockClock, SystemClock};

#[test]
fn test_system_clock() {
    let clock = SystemClock;
    let now1 = clock.now();
    std::thread::sleep(Duration::from_millis(2));
    let now2 = clock.now();
    assert!(now2 > now1);
}

#[test]
fn test_mock_clock_default_and_advance() {
    let mut clock = MockClock::default();
    let now1 = clock.now();
    clock.advance(Duration::from_secs(5));
    let now2 = clock.now();
    assert_eq!(now2.duration_since(now1), Duration::from_secs(5));
}

#[test]
fn test_box_clock() {
    let clock: Box<dyn Clock> = Box::new(MockClock::new());
    let now = clock.now();
    assert!(now <= Instant::now());
}

#[test]
fn test_arc_mutex_clock() {
    let mock = MockClock::new();
    let clock = Arc::new(Mutex::new(mock));
    let now = clock.now();
    assert!(now <= Instant::now());
}
