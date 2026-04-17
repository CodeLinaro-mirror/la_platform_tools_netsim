// Copyright 2023-2025 The Android Open Source Project

//! Uses the tracing crate that allows control of logging through
//! the RUST_LOG environment variable.

/// Initiating the environment for logging with given prefix
///
/// The current log format follows the same format as Android Emulator team.
pub fn init(prefix: &'static str, is_verbose: bool) {
    common::util::netsim_logger::init(prefix, is_verbose);
}
