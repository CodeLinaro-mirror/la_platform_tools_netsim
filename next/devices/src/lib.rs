// Copyright 2023-2025 The Android Open Source Project

//! # Netsim Device Service
//!
//! This crate provides the `DeviceServer`, which manages the lifecycle of simulated devices
//! and their associated chips. It interacts with chip-specific services (like Bluetooth, WiFi)
//! via `ChipClient` handles.
//!
//! # Getting Started
//!
//! The main entry point for interacting with this crate is the [`Server`].
//!
//! To use the `Server`:
//! 1. Create a new instance using [`Server::new()`], which returns the `Server` and a `DeviceClient`.
//! 2. Create clients for all required chip services (e.g., Bluetooth, WiFi).
//! 3. Collect the chip clients into a `HashMap`.
//! 4. Spawn the [`Server::run()`] method into a Tokio task, passing the `chip_clients` map.
//! 5. Use the `DeviceClient` to send commands to the running `Server`.
//!
//! ```no_run
//! use bluetooth;
//! use devices::Server;
//! use tokio;
//! use netsim_api::chips::{ChipClient, NetworkKind};
//! use netsim_api::devices::{DeviceClient, DeviceRequest};
//! use std::collections::HashMap;
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 1. Create the DeviceServer and its client
//!     let (device_server, device_client) = Server::new(false);
//!
//!     // 2. Create a Bluetooth chip server and client, passing the device_client
//!     let (bt_server, bt_client) = bluetooth::Server::new(device_client.clone());
//!     //    Spawn the Bluetooth server to run in the background.
//!     tokio::spawn(async move {
//!         bt_server.run().await;
//!     });
//!
//!     // 3. Create the map of chip clients
//!     let mut chip_clients = HashMap::new();
//!     chip_clients.insert(NetworkKind::Bluetooth, bt_client);
//!     //    Add other chip clients (WiFi, Cell, etc.) here
//!
//!     // 4. Spawn the device server.
//!     tokio::spawn(async move {
//!         device_server.run(chip_clients).await;
//!     });
//!
//!     // Use the client to interact with the server, e.g., create devices.
//!     // device_client.create(...).await;
//!
//!     // Keep the main task alive for a duration or until a shutdown signal.
//!     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//! }
//! ```
//!
//! # Commands
//!
//! The `Server` processes commands sent via the `DeviceClient`. The available
//! commands are defined in the [`netsim_api::devices::DeviceRequest`] enum. The
//! `DeviceClient` provides a convenient method for each command variant.
//!
//! The following commands are supported:
//! - [`PsCreate`](netsim_api::devices::DeviceClient::ps_create): Creates a new device from packet streamer parameters.
//! - [`Create`](netsim_api::devices::DeviceClient::create): Creates a new device.
//! - [`List`](netsim_api::devices::DeviceClient::list): Retrieves information about all devices.
//! - [`Update`](netsim_api::devices::DeviceClient::update): Updates an existing device.
//! - [`Delete`](netsim_api::devices::DeviceClient::delete): Deletes a device.
//! - [`Reset`](netsim_api::devices::DeviceRequest::Reset): Resets all devices.
//! - [`GetChipStatistics`](netsim_api::devices::DeviceRequest::GetChipStatistics): Retrieves statistics for all chips.
//! - [`Shutdown`](netsim_api::devices::DeviceRequest::Shutdown): Shuts down the `Server`.
//!
//! # Features
//!
//! * **Actor-Based State Management:** Implements the actor model, with the `Server` as a central
//!   actor that serializes all operations to safely manage the state of multiple
//!   devices and their associated chips.
//! * **Unique ID Generation:** Vends unique `ChipId` and `DeviceId` to ensure
//!   uniqueness across the simulation.
//! * **Chip Service Integration:** Interacts with chip-specific services (e.g., Bluetooth)
//!   to manage the lifecycle of individual chips.

/// This module contains the implementation of the command handlers for the `Server`.
pub mod handlers;
pub mod server;

pub use server::Server;
