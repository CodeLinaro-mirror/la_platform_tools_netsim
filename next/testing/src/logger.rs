// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the logger for testing.
/// This function ensures that the logger is initialized only once.
/// It uses `tracing-subscriber` to print logs to stdout/stderr.
pub fn setup(_level: Option<&str>) {
    INIT.call_once(|| {
        // For now, we just call init_for_test which uses "info"
        // If we need custom level, we should update common::util::netsim_logger
        common::util::netsim_logger::init_for_test();
    });
}
