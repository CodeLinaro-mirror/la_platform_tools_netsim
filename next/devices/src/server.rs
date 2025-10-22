// Copyright 2023-2025 The Android Open Source Project

//! # Device Service
//!
//! This module implements the device service, which is responsible for managing the
//! lifecycle of emulated devices within the simulation. A device is a logical
//! grouping of one or more radio chips.
//!
//! The service handles device creation, deletion, and state management. It also
//! acts as a central authority for vending `ChipId` and `DeviceId` to ensure
//! uniqueness across the simulation.
//!
//! ## Architecture
//!
//! The `Server` struct is the core of the service. It listens for `DeviceRequest`
//! messages on a channel and processes them in a loop. It maintains the state of
//! all devices and chips in the simulation.
//!
//! The service interacts with chip-specific services (e.g., the Bluetooth service)
//! to manage the lifecycle of individual chips.
//!
//! ## Lifecycle
//!
//! The device service starts up and remains idle until it receives a request.
//! It has a configurable idle timeout. If no requests are received within the
//! timeout period, the service will shut down to conserve resources.

use log::info;
use netsim_api::chips::{ChipClient, ChipId, NetworkKind};
use netsim_api::device_error::DeviceError;
use netsim_api::devices::{DeviceClient, DeviceConfig, DeviceId, DeviceRequest};
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{self, Instant, Sleep};

/// The central server for the device service.
///
/// This struct maintains the state of all simulated devices and chips.
pub struct Server {
    /// A client for interacting with the Bluetooth chip service.
    pub bt_client: ChipClient,
    /// The next available `ChipId`.
    next_chip_id: AtomicU32,
    /// The next available `DeviceId`.
    next_device_id: AtomicU32,
    /// A map of all devices, keyed by `DeviceId`.
    pub devices_by_id: HashMap<DeviceId, DeviceInfo>,
    /// A map from device GUID to `DeviceId`.
    pub device_ids_by_guid: HashMap<String, DeviceId>,
    /// A map from `ChipId` to the device it belongs to.
    pub chip_to_device_map: HashMap<ChipId, (NetworkKind, DeviceId)>,
    /// The receiver for incoming `DeviceRequest` messages.
    request_rx: mpsc::Receiver<DeviceRequest>,
    /// A timer for shutting down the service when idle.
    shutdown_alarm: Pin<Box<Sleep>>,
    /// The initial timeout before the service shuts down if no requests are received.
    pub start_timeout: Duration,
    /// The idle timeout before the service shuts down.
    pub idle_timeout: Duration,
}

/// Represents a single simulated device.
///
/// A device is a collection of chips and has properties such as position and orientation.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// The unique identifier for the device.
    pub id: DeviceId,
    /// A GUID for the device, typically provided by the packet streamer.
    pub guid: String,
    pub chips: HashSet<ChipId>,
    /// Device configuration.
    pub device_config: DeviceConfig,
}

impl Server {
    /// Creates a new `Server` and a corresponding `DeviceClient`.
    pub fn new(bt_client: ChipClient) -> (Self, DeviceClient) {
        let (command_tx, request_rx) = mpsc::channel(10);

        let server = Server {
            bt_client,
            next_chip_id: AtomicU32::new(0),
            next_device_id: AtomicU32::new(0),
            request_rx,
            devices_by_id: HashMap::new(),
            device_ids_by_guid: HashMap::new(),
            chip_to_device_map: HashMap::new(),
            shutdown_alarm: Box::pin(time::sleep_until(Instant::now())),
            // TODO: Pass timeouts on new()
            start_timeout: Duration::from_secs(15),
            idle_timeout: Duration::from_secs(15),
        };
        (server, DeviceClient::new(command_tx))
    }

    /// Runs the device service's main loop.
    ///
    /// The server will listen for incoming requests and handle them accordingly.
    /// It will shut down if it remains idle for the configured timeout.
    pub async fn run(mut self) {
        self.set_alarm(self.start_timeout);
        loop {
            tokio::select! {
                Some(cmd) = self.request_rx.recv() =>
                self.handle_command(cmd).await,
                _ = &mut self.shutdown_alarm => break,
            }
        }
        info!("Device server is shutdown");
    }

    fn set_alarm(&mut self, duration: Duration) {
        let new_deadline = Instant::now() + duration;
        self.shutdown_alarm.as_mut().reset(new_deadline);
    }

    /// Generates a new, unique `ChipId`.
    pub fn new_chip_id(&self) -> ChipId {
        let id = self.next_chip_id.fetch_add(1, Ordering::SeqCst);
        ChipId(id)
    }

    /// Generates a new, unique `DeviceId`.
    pub fn new_device_id(&self) -> DeviceId {
        let id = self.next_device_id.fetch_add(1, Ordering::SeqCst);
        DeviceId(id)
    }

    pub(crate) fn get_device_info(
        &mut self,
        id: &DeviceId,
    ) -> Result<&mut DeviceInfo, DeviceError> {
        self.devices_by_id
            .get_mut(id)
            .ok_or_else(|| DeviceError::Internal(format!("Device {id}'s info not found")))
    }
}
