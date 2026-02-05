// Copyright 2026 The Android Open Source Project

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the logger for testing.
/// This function ensures that the logger is initialized only once.
/// It uses `env_logger` to print logs to stdout/stderr.
pub fn setup(level: Option<&str>) {
    INIT.call_once(|| {
        let mut builder = env_logger::builder();
        builder.is_test(true);

        if let Some(l) = level {
            builder.parse_filters(l);
        } else {
            // Default to info if not specified
            if std::env::var("RUST_LOG").is_err() {
                builder.filter_level(log::LevelFilter::Info);
            } else {
                builder.parse_default_env();
            }
        }

        let _ = builder.try_init();
    });
}
