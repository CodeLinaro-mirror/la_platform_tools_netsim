// Copyright (C) 2025 The Android Open Source Project

//! The bluetooth crate provides a simulation environment for Bluetooth devices.
//!
//! This crate is responsible for managing the lifecycle of simulated Bluetooth
//! chips, handling HCI communication, and interacting with the `rootcanal`
//! simulation backend.
#![allow(missing_docs)]
#![allow(clippy::type_complexity)]

mod beacon;
mod device;
mod handlers;
pub mod server;
mod sniffer;
mod utils;

pub use server::Server;
