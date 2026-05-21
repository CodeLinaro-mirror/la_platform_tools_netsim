// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::process;

#[cfg(all(target_os = "linux", feature = "cuttlefish"))]
use command_fds::inherited::init_inherited_fds;
use daemon::{RunResult, run};

fn main() {
    // SAFETY: This is the first line of the main function before anything opens
    // files or otherwise, takes ownership of any file descriptors.
    #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
    unsafe {
        init_inherited_fds();
    }
    match run() {
        RunResult::ExitedNormally => process::exit(0),
        RunResult::InitializationError(e) => {
            eprintln!("Netsim daemon failed to start: {}", e);
            process::exit(1);
        }
    }
}
