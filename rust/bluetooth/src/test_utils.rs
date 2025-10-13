// Copyright 2023-2025 The Android Open Source Project

//! This module provides testing utility functions for the bluetooth crate.

use env_logger::{Builder, Target};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// Initializes env_logger for tests, defaulting to DEBUG level if RUST_LOG is not set.
pub fn setup_logging() {
    let _ = Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format(|buf, record| {
            let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            writeln!(
                buf,
                "{}s [{}] {}:{} - {}",
                secs,
                record.level(),
                record.module_path().unwrap_or("<unknown>"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .target(Target::Stderr) // Logs to stderr, visible in Blaze test output
        .try_init(); // Use try_init to avoid panicking if called multiple times in parallel tests
}
