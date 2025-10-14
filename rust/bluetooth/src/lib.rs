// Copyright (C) 2025 The Android Open Source Project

//! The bluetooth crate provides a simulation environment for Bluetooth devices.
//!
//! This crate is responsible for managing the lifecycle of simulated Bluetooth
//! chips, handling HCI communication, and interacting with the `rootcanal`
//! simulation backend.
#![allow(missing_docs)]

mod beacon;
mod handlers;
pub mod manager;
mod sniffer;
#[cfg(feature = "test-utils")]
pub mod test_utils;
mod types;
pub mod utils;
mod virtual_device;

pub use manager::BluetoothManager;
