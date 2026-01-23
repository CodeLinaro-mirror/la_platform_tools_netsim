// Copyright 2023-2025 The Android Open Source Project

//! A logger for use by netsimd.
//!
//! Uses the env_logger crate that allows control of logging through
//! the RUST_LOG environment variable.

use chrono::Utc;
use env_logger::{Builder, Env};
use log::{Level, Record};
use std::{io::Write, path::Path};

/// Formats the current time for logging.
fn log_current_time() -> String {
    Utc::now().format("%m-%d %H:%M:%S%.3f").to_string()
}

/// Initiating the environment for logging with given prefix
///
/// The current log format follows the same format as Android Emulator team.
pub fn init(prefix: &'static str, is_verbose: bool) {
    let log_filter = if is_verbose { "debug" } else { "info" };
    let mut builder = Builder::from_env(Env::default().default_filter_or(log_filter));
    builder.format(move |buf, record| {
        let level = level_to_string(record.level());
        let message = format!(
            "{} {} {} {}:{} - {}",
            prefix,
            level,
            log_current_time(),
            format_file(record),
            record.line().unwrap_or(0),
            record.args()
        );
        writeln!(buf, "{message}")
    });
    builder.try_init().ok();
}

/// Helper function for parsing the file name from given record file path
/// This will provide the file information where the log function is called
/// NOTE: This implementation is fragile as it relies on string matching.
/// A change in the build environment's directory structure could break it.
fn format_file<'a>(record: &'a Record<'a>) -> &'a str {
    match record.file() {
        Some(filepath) => {
            Path::new(filepath).file_name().map(|s| s.to_str().unwrap_or("N/A")).unwrap_or("N/A")
        }
        None => "N/A",
    }
}

/// Helper function for translating log levels to string.
fn level_to_string(level: Level) -> &'static str {
    match level {
        Level::Error => "E",
        Level::Warn => "W",
        Level::Info => "I",
        Level::Debug => "D",
        Level::Trace => "T",
    }
}

#[cfg(test)]
// NOTE: These tests are basic. A more robust implementation would capture
// the logger's output and assert its format and content.
mod tests {
    use super::*;
    use log::LevelFilter;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup_logger() {
        INIT.call_once(|| {
            // Initialize env_logger for tests, but only once.
            // This prevents "logger already initialized" errors.
            let _ = env_logger::builder().is_test(true).filter_level(LevelFilter::Debug).try_init();
        });
    }

    #[test]
    fn test_init_does_not_panic() {
        setup_logger();
        // init can be called multiple times in tests if try_init is used,
        // but the actual logger will only be set up once.
        // This test primarily checks that the init function itself doesn't panic.
        init("TEST_PREFIX", false);
    }

    #[test]
    fn test_format_file_shortens_paths() {
        let record = log::Record::builder()
            .file(Some("./external/netsim+/next/daemon/src/netsimd.rs"))
            .build();
        assert_eq!(format_file(&record), "netsimd.rs");

        let record2 = log::Record::builder()
            .file(Some("external/netsim+/next/daemon/src/netsimd.rs"))
            .build();
        assert_eq!(format_file(&record2), "netsimd.rs");
    }
}
