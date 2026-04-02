// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Netsim Next
//!
//! This crate provides the main binary for the netsim daemon, `netsimd`.
//! It is responsible for handling frontend requests and managing the simulation
//! state.
//!
//! ## Internal Module Structure
//!
//! Here is a dependency graph of the internal modules:
//!
//! ![Internal Module Graph](modules.png)

pub(crate) mod args;
pub(crate) mod avd_config;
pub(crate) mod ini_file;
pub(crate) mod logger;
pub(crate) mod netsimd;
pub(crate) mod version;

pub use args::{Args, ClapWifiMode};
pub use ini_file::{IniFile, IniFileAccess, IniFileInitialized};
pub use netsimd::{run, NetsimDaemon, RunResult, StartUpMode};
