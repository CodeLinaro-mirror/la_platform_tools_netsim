// Copyright (C) 2025 The Android Open Source Project

//! The bluetooth crate provides a simulation environment for Bluetooth devices.
//!
//! This crate is responsible for managing the lifecycle of simulated Bluetooth
//! chips, handling HCI communication, and interacting with the `rootcanal`
//! Bluetooth emulator. simulation backend.
//!
//! # Getting Started
//!
//! The main entry point for interacting with this crate is the [`new`]
//! function.
//!
//! To use the bluetooth actor:
//! 1. Create a new instance using [`new()`], which returns the `ResourceActor`
//!    and a `ChipClient`.
//! 2. Spawn the `actor.run(context)` method into a Tokio task to start its
//!    event loop.
//! 3. Use the `ChipClient` to send commands to the running actor.
//!
//! ```no_run
//! use netsim_model::device::{DeviceClient, DeviceRequest};
//! use tokio::{self, sync::mpsc};
//!
//! #[tokio::main]
//! async fn main() {
//!     let (device_tx, _device_rx) = mpsc::channel(10);
//!     let resource_client = actor_framework::ResourceClient::new(device_tx);
//!     // device_client creation depends on where DeviceClient comes from.
//!     // Assuming client::DeviceClient is correct based on bluetooth_actor.rs usage.
//!     let device_client = client::DeviceClient::new(Box::new(resource_client));
//!     let (actor, client) = bluetooth_actor::new();
//!     let bluetooth_actor = bluetooth_actor::BluetoothActor::new(device_client);
//!     tokio::spawn(async move {
//!         actor.run(bluetooth_actor).await;
//!     });
//!
//!     // Use the client to interact with the actor, e.g., create chips.
//!     // client.create(...).await;
//!
//!     // Keep the main task alive for a duration or until a shutdown signal.
//!     tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
//! }
//! ```
//!
//! # Commands
//!
//! The actor processes commands sent via the `ChipClient`. The available
//! commands are defined in the [`netsim_model::chip::ChipRequest`] enum. The
//! `ChipClient` provides a convenient method for each command variant.
//!
//! The following commands are supported:
//! - [`Create`](netsim_model::chip::ChipClient::create): Creates a new
//!   Bluetooth chip.
//! - [`Read`](netsim_model::chip::ChipClient::read): Retrieves information
//!   about a Bluetooth chip.
//! - [`Update`](netsim_model::chip::ChipClient::update): Updates an existing
//!   Bluetooth chip.
//! - [`Delete`](netsim_model::chip::ChipClient::delete): Deletes a Bluetooth
//!   chip.
//! - [`Reset`](netsim_model::chip::ChipClient::reset): Resets a Bluetooth chip.
//! - [`GetStatistics`](netsim_model::chip::ChipRequest::GetStatistics):
//!   Retrieves statistics for all Bluetooth chips.
//! - [`GetCountForTesting`](netsim_model::chip::ChipRequest::GetCountForTesting): Retrieves the total number of chips for testing purposes.
//! - [`Shutdown`](netsim_model::chip::ChipClient::shutdown): Shuts down the
//!   actor.
//!
//! # Chip Modes
//!
//! The actor can create chips in three different modes, configured via the
//! [`netsim_model::chip::ChipCreate`] struct. The `mode` field within
//! [`netsim_model::chip::BluetoothCreate`] determines the chip's behavior.
//!
//! ## Device Mode
//!
//! A standard Bluetooth controller that can be controlled by an external host,
//! such as an Android Virtual Device or a Bumble test. This is configured
//! using [`netsim_model::chip::BluetoothMode::Device`].
//!
//! ## Beacon Mode
//!
//! A chip that repeatedly broadcasts advertisement packets. This is configured
//! using [`netsim_model::chip::BluetoothMode::Beacon`] with
//! [`netsim_model::chip::BeaconParams`].
//!
//! ## Scanner Mode
//!
//! A chip that listens for and captures all nearby Bluetooth packets. This is
//! configured using [`netsim_model::chip::BluetoothMode::Scanner`].
//!
//! # Features
//!
//! * **Actor-Based State Management:** Implements the actor model, with the
//!   `ResourceActor` as a central actor that serializes all operations to
//!   safely manage the state of multiple Bluetooth chips (Device, Beacon, and
//!   Scanner modes).
//! * **HCI Stream/Sink Bridging:** For each chip, bridges a `PacketStream` (for
//!   incoming HCI commands) and a `PacketSink` (for outgoing HCI events),
//!   routing packets between the host and the `rootcanal` simulation.
//! * **Rootcanal Integration:** Simulates the Bluetooth controller logic using
//!   `rootcanal`.
//!
//! # Future Features
//!
//! * **RSSI Management:** Manage Received Signal Strength Indication (RSSI)
//!   based on chip location.
//! * **Link Layer Capture:** Scanner functionality to convert Rootcanal LL
//!   packets to standard Bluetooth LL packets.
//! * **HCI-based Beacon:** Implement Beacon functionality via HCI commands,
//!   allowing common Android-like advertisement parameters.
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod actions;
mod beacon;
mod bluetooth_actor; // Renamed from actor
mod device;
mod error;
mod hci_callbacks; // Added

mod actor_service;
mod lifecycle; // Added
mod ranging;
mod scanner;
mod utils;

/// Utilities for Bluetooth Beacons and advertising data
pub mod beacon_utils;
pub mod client;

pub use actions::{BluetoothAction, BluetoothActionResult};
pub use bluetooth_actor::BluetoothActor;
pub use client::BluetoothClient;
pub use error::BluetoothError;
/// The entity type managed by the Bluetooth ResourceActor.
pub type BluetoothEntity = BluetoothActor;

use actor_framework::ResourceActor;

/// Creates a new Bluetooth actor and its client.
pub fn new() -> (ResourceActor<BluetoothEntity>, BluetoothClient) {
    let (actor, resource_client) = ResourceActor::new(32);
    (actor, BluetoothClient(resource_client))
}

// TODO: Consider generic impl<T> ChipClient for ResourceClient<T>.
// Implementation moved to client.rs
