// Copyright 2023-2025 The Android Open Source Project

//! # Netsim Next
//!
//! This crate provides the main binary for the netsim daemon, `netsimd`.
//! It is responsible for handling frontend requests and managing the simulation state.
//!
//! ## Internal Module Structure
//!
//! Here is a dependency graph of the internal modules:
//!
//! ![Internal Module Graph](modules.png)

pub mod args;
pub mod config;
pub mod logger;
pub mod netsimd;
pub mod platform;
