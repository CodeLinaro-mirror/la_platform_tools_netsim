// Copyright 2023-2025 The Android Open Source Project

//! This module defines the public data structures and traits for the Bluetooth
//! manager.

mod handlers;
mod manager_impl;
pub(crate) mod types;

pub use manager_impl::BluetoothManager;
pub use types::{BluetoothCommand, ChipState};
