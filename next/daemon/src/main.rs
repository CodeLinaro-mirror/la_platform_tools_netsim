// Copyright 2023-2025 The Android Open Source Project

use daemon::netsimd::{run, RunResult};
use std::process;

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
