// Copyright 2023-2025 The Android Open Source Project

//! The devices crate provides a simulation environment for devices.
//!
//! A device is a logical grouping of one or more radio chips (e.g. Bluetooth, Wi-Fi).
//! This crate is responsible for managing the lifecycle of simulated devices
//! and dispatching chip-level requests to the appropriate chip services.
//!
//! # Getting Started
//!
//! The main entry point for interacting with this crate is the [`Server`].
//!
//! To use the `Server`:
//! 1. Create a new instance using [`Server::new()`], which returns the `Server` and a `DeviceClient`.
//! 2. Spawn the [`Server::run()`] method into a Tokio task to start its event loop.
//! 3. Use the `DeviceClient` to send commands to the running `Server`.
//!
//! ```no_run
//! use bluetooth;
//! use devices::Server;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 1. Create a Bluetooth chip server and client.
//!     let (bt_server, bt_client) = bluetooth::Server::new();
//!     // 2. Spawn the Bluetooth server to run in the background.
//!     tokio::spawn(async move {
//!         bt_server.run().await;
//!     });
//!
//!     // 3. Create the device server, providing it with the Bluetooth client.
//!     let (server, client) = Server::new(bt_client, false);
//!     // 4. Spawn the device server.
//!     tokio::spawn(async move {
//!         server.run().await;
//!     });
//!
//!     // Use the client to interact with the server, e.g., create devices.
//!     // client.create(...).await;
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
//! - [`PsCreate`][netsim_api::devices::DeviceClient::ps_create]: Creates a new device from packet streamer parameters.
//! - [`Create`][netsim_api::devices::DeviceRequest::Create]: Creates a new device.
//! - [`List`][netsim_api::devices::DeviceClient::list]: Retrieves information about all devices.
//! - [`Update`][netsim_api::devices::DeviceRequest::Update]: Updates an existing device.
//! - [`Delete`][netsim_api::devices::DeviceRequest::Delete]: Deletes a device.
//! - [`Reset`][netsim_api::devices::DeviceRequest::Reset]: Resets all devices.
//! - [`GetChipStatistics`][netsim_api::devices::DeviceRequest::GetChipStatistics]: Retrieves statistics for all chips.
//! - [`Shutdown`][netsim_api::devices::DeviceRequest::Shutdown]: Shuts down the `Server`.
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
