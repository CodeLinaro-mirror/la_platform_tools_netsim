// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::process;

use daemon::netsimd::{run, RunResult};

#[tokio::main]
async fn main() {
    match run().await {
        RunResult::ExitedNormally => process::exit(0),
        RunResult::InitializationError(e) => {
            eprintln!("Netsim daemon failed to start: {}", e);
            process::exit(1);
        }
    }
}
