// Copyright (C) 2025 The Android Open Source Project

//! The bluetooth crate provides a simulation environment for Bluetooth devices.
//!
//! This crate is responsible for managing the lifecycle of simulated Bluetooth
//! chips, handling HCI communication, and interacting with the `rootcanal` Bluetooth emulator.
//! simulation backend.
//!
//! # Getting Started
//!
//! The main entry point for interacting with this crate is the [`new`] function.
//!
//! To use the bluetooth actor:
//! 1. Create a new instance using [`new()`], which returns the `ResourceActor` and a `ChipClient`.
//! 2. Spawn the `actor.run(context)` method into a Tokio task to start its event loop.
//! 3. Use the `ChipClient` to send commands to the running actor.
//!
//! ```no_run
//! use tokio;
//! use netsim_model::device::{DeviceClient, DeviceRequest};
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() {
//!     let (device_tx, _device_rx) = mpsc::channel(10);
//!     let resource_client = actor_framework::ResourceClient::new(device_tx);
//!     // device_client creation depends on where DeviceClient comes from.
//!     // Assuming client::DeviceClient is correct based on bluetooth_actor.rs usage.
//!     let device_client = client::DeviceClient::new(resource_client);
//!     let (actor, client) = bluetooth::new();
//!     let bluetooth_actor = bluetooth::BluetoothActor::new(device_client, client.clone());
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
//! - [`Create`](netsim_model::chip::ChipClient::create): Creates a new Bluetooth chip.
//! - [`Read`](netsim_model::chip::ChipClient::read): Retrieves information about a Bluetooth chip.
//! - [`Update`](netsim_model::chip::ChipClient::update): Updates an existing Bluetooth chip.
//! - [`Delete`](netsim_model::chip::ChipClient::delete): Deletes a Bluetooth chip.
//! - [`Reset`](netsim_model::chip::ChipClient::reset): Resets a Bluetooth chip.
//! - [`GetStatistics`](netsim_model::chip::ChipRequest::GetStatistics): Retrieves statistics for all Bluetooth chips.
//! - [`GetCountForTesting`](netsim_model::chip::ChipRequest::GetCountForTesting): Retrieves the total number of chips for testing purposes.
//! - [`Shutdown`](netsim_model::chip::ChipClient::shutdown): Shuts down the actor.
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
//! using [`netsim_model::chip::BluetoothMode::Beacon`] with [`netsim_model::chip::BeaconParams`].
//!
//! ## Sniffer Mode
//!
//! A chip that listens for and captures all nearby Bluetooth packets. This is
//! configured using [`netsim_model::chip::BluetoothMode::Sniffer`].
//!
//! # Features
//!
//! * **Actor-Based State Management:** Implements the actor model, with the `ResourceActor` as a central
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
#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod actions;
mod beacon;
mod bluetooth_actor; // Renamed from actor
mod device;
mod error;
mod hci_callbacks; // Added
mod internal_chip; // Added
mod lifecycle; // Added
mod ranging;
mod service;
mod sniffer;
mod utils;

pub use actions::{BluetoothAction, BluetoothActionResult};
pub use bluetooth_actor::BluetoothActor;
pub use error::BluetoothError;
/// The entity type managed by the Bluetooth ResourceActor.
pub type BluetoothEntity = BluetoothActor;

use actor_framework::{ResourceActor, ResourceClient};

use netsim_model::chip::{ChipClient, ChipCreate, ChipId};

/// A client for the Bluetooth actor.
#[derive(Clone)]
pub struct BluetoothClient(pub ResourceClient<BluetoothEntity>);

/// Creates a new Bluetooth actor and its client.
pub fn new() -> (ResourceActor<BluetoothEntity>, BluetoothClient) {
    let (actor, resource_client) = ResourceActor::new(32);
    (actor, BluetoothClient(resource_client))
}

// TODO: Consider generic impl<T> ChipClient for ResourceClient<T>.
#[async_trait::async_trait]
impl ChipClient for BluetoothClient {
    async fn create(
        &self,
        params: ChipCreate,
    ) -> Result<(), netsim_model::client_error::ClientError> {
        self.0
            .create(params)
            .await
            .map(|_| ())
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))
    }

    async fn read(
        &self,
        id: ChipId,
    ) -> Result<netsim_model::chip::Chip, netsim_model::client_error::ClientError> {
        self.0
            .get(id)
            .await
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))?
            .ok_or(netsim_model::client_error::ClientError::Chip(
                netsim_model::chip_error::ChipError::ChipNotFound(id),
            ))
    }

    async fn update(
        &self,
        id: ChipId,
        patch: netsim_model::chip::ChipUpdate,
    ) -> Result<netsim_model::chip::Chip, netsim_model::client_error::ClientError> {
        self.0
            .update(id, patch)
            .await
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))
    }

    async fn delete(&self, id: ChipId) -> Result<(), netsim_model::client_error::ClientError> {
        self.0
            .delete(id)
            .await
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))
    }

    async fn read_statistics(
        &self,
    ) -> Result<Vec<netsim_model::stats::NetsimRadioStats>, netsim_model::client_error::ClientError>
    {
        // Workaround: GetStatistics is an action, but requires an ID.
        // We list chips first. If empty, return empty stats.
        // If not empty, use the first chip ID to invoke the action (which returns global stats).
        let chips = self
            .0
            .list()
            .await
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))?;
        if chips.is_empty() {
            return Ok(Vec::new());
        }
        let first_id = chips[0].id;
        match self.0.perform_action(Some(ChipId(first_id)), BluetoothAction::GetStatistics).await {
            Ok(BluetoothActionResult::Statistics(stats)) => Ok(stats),
            Ok(_) => Err(netsim_model::client_error::ClientError::Recv(
                "Unexpected action result".into(),
            )),
            Err(e) => Err(netsim_model::client_error::ClientError::Send(e.to_string())),
        }
    }

    async fn read_count_for_testing(
        &self,
    ) -> Result<usize, netsim_model::client_error::ClientError> {
        self.0
            .list()
            .await
            .map(|chips| chips.len())
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))
    }

    async fn shutdown(&self) -> Result<(), netsim_model::client_error::ClientError> {
        // ResourceClient does not support explicit shutdown.
        // Dropping the client will eventually shut down the actor if it's the last one.
        Ok(())
    }

    async fn reset(&self, id: ChipId) -> Result<(), netsim_model::client_error::ClientError> {
        self.0
            .perform_action(Some(id), BluetoothAction::Reset { id })
            .await
            .map(|_| ())
            .map_err(|e| netsim_model::client_error::ClientError::Send(e.to_string()))
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
