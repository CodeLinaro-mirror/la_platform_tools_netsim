//  Copyright 2023 The Android Open Source Project
//
// SPDX-License-Identifier: Apache-2.0

//! # netsim-common Crate
//!
//! A collection of utilities for netsimd and netsim.

pub mod system;
pub mod util;

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    // Shared mutex to ensure environment is not modified by multiple tests simultaneously
    pub static ENV_MUTEX: Mutex<()> = Mutex::new(());
}
