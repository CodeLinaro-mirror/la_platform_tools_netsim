// Copyright 2023-2025 The Android Open Source Project

use netsim_next::netsimd::run;
use std::process;

#[tokio::main]
async fn main() {
    if !run().await {
        process::exit(1);
    }
}
