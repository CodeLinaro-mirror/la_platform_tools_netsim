// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// tests/common/mod.rs

static LOG_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_logger() {
    LOG_INIT.call_once(|| {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    });
}
