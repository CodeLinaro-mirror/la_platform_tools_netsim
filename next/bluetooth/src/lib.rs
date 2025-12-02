// Copyright (C) 2025 The Android Open Source Project

//! The bluetooth crate provides a simulation environment for Bluetooth devices.
//!
//! This crate is responsible for managing the lifecycle of simulated Bluetooth
//! chips, handling HCI communication, and interacting with the `rootcanal` Bluetooth emulator.
//! simulation backend.
//!
//! # Getting Started
//!
//! The main entry point for interacting with this crate is the [`Server`].
//!
//! To use the `Server`:
//! 1. Create a new instance using [`Server::new()`], which returns the `Server` and a `ChipClient`.
//! 2. Spawn the [`Server::run()`] method into a Tokio task to start its event loop.
//! 3. Use the `ChipClient` to send commands to the running `Server`.
//!
//! ```no_run
//! use tokio;
//! use netsim_api::devices::{DeviceClient, DeviceRequest};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let (device_tx, _device_rx) = mpsc::channel::<DeviceRequest>(10);
//!     let device_client = DeviceClient::new(device_tx);
//!     let (server, client) = bluetooth::Server::new(device_client);
//!     tokio::spawn(async move {
//!         server.run().await;
//!     });
//!
//!     // Use the client to interact with the server, e.g., create chips.
//!     // client.create(...).await;
//!
//!     // Keep the main task alive for a duration or until a shutdown signal.
//!     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//! }
//! ```
//!
//! # Commands
//!
//! The `Server` processes commands sent via the `ChipClient`. The available
//! commands are defined in the [`netsim_api::chips::ChipRequest`] enum. The
//! `ChipClient` provides a convenient method for each command variant.
//!
//! The following commands are supported:
//! - [`Create`](netsim_api::chips::ChipClient::create): Creates a new Bluetooth chip.
//! - [`Read`](netsim_api::chips::ChipClient::read): Retrieves information about a Bluetooth chip.
//! - [`Update`](netsim_api::chips::ChipClient::update): Updates an existing Bluetooth chip.
//! - [`Delete`](netsim_api::chips::ChipClient::delete): Deletes a Bluetooth chip.
//! - [`Reset`](netsim_api::chips::ChipClient::reset): Resets a Bluetooth chip.
//! - [`GetStatistics`](netsim_api::chips::ChipRequest::GetStatistics): Retrieves statistics for all Bluetooth chips.
//! - [`GetCountForTesting`](netsim_api::chips::ChipRequest::GetCountForTesting): Retrieves the total number of chips for testing purposes.
//! - [`Shutdown`](netsim_api::chips::ChipClient::shutdown): Shuts down the `Server`.
//!
//! # Chip Modes
//!
//! The `Server` can create chips in three different modes, configured via the
//! [`netsim_api::chips::CreateParams`] struct. The `mode` field within
//! [`netsim_api::chips::BluetoothParams`] determines the chip's behavior.
//!
//! ## Device Mode
//!
//! A standard Bluetooth controller that can be controlled by an external host,
//! such as an Android Virtual Device or a Bumble test. This is configured
//! using [`netsim_api::chips::BluetoothMode::Device`].
//!
//! ## Beacon Mode
//!
//! A chip that repeatedly broadcasts advertisement packets. This is configured
//! using [`netsim_api::chips::BluetoothMode::Beacon`] with [`netsim_api::chips::BeaconParams`].
//!
//! ## Sniffer Mode
//!
//! A chip that listens for and captures all nearby Bluetooth packets. This is
//! configured using [`netsim_api::chips::BluetoothMode::Sniffer`].
//!
//! # Features
//!
//! * **Actor-Based State Management:** Implements the actor model, with the `Server` as a central
//!   actor that serializes all operations to safely manage the state of multiple Bluetooth
//!   chips (Device, Beacon, and Sniffer modes).
//! * **HCI Stream/Sink Bridging:** For each chip, bridges a `PacketStream` (for incoming HCI
//!   commands) and a `PacketSink` (for outgoing HCI events), routing packets between the host
//!   and the `rootcanal` simulation.
//! * **Rootcanal Integration:** Simulates the Bluetooth controller logic using `rootcanal`.
//!
//! # Future Features
//!
//! * **RSSI Management:** Manage Received Signal Strength Indication (RSSI) based on chip location.
//! * **Link Layer Capture:** Sniffer functionality to convert Rootcanal LL packets to standard Bluetooth LL packets.
//! * **HCI-based Beacon:** Implement Beacon functionality via HCI commands, allowing common Android-like advertisement parameters.
#![deny(missing_docs)]
#![allow(clippy::type_complexity)]

mod beacon;
mod device;
mod handlers;
pub mod ranging;
pub mod server;
mod sniffer;
mod utils;

pub use server::Server;
