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

const DEFAULT_START_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TIMEOUT: Duration = Duration::from_secs(u32::MAX as u64);

/// The `Server` is the central actor in the device service, responsible for
/// managing the state of all simulated devices and their associated chips.
///
/// It is implemented as a Tokio actor that processes requests in a serialized
/// manner. This design ensures that all state modifications are thread-safe
/// without requiring locks or other synchronization primitives.
pub struct Server {
    // TODO: change pub to pub(crate) everywhere. Actors only export a message API.
    /// A map of clients for interacting with technology-specific chip services.
    pub chip_clients: HashMap<NetworkKind, ChipClient>,
    /// The next available `ChipId`.
    next_chip_id: AtomicU32,
    /// The next available `DeviceId`.
    next_device_id: AtomicU32,
    /// A map of all devices, keyed by `DeviceId`.
    pub devices_by_id: HashMap<DeviceId, DeviceInfo>,
    /// Maps GUIDs to `DeviceId`s. Used to re-identify emulators devices for adding chips. Accessory devices don't use this.
    pub device_ids_by_guid: HashMap<String, DeviceId>,
    /// A map from `ChipId` to its associated `ChipInfo`.
    pub chip_info_map: HashMap<ChipId, ChipInfo>,
    /// The receiver for incoming `DeviceRequest` messages.
    request_rx: mpsc::Receiver<DeviceRequest>,
    /// A timer for shutting down the service when idle.
    ///
    /// The `shutdown_alarm` is reset every time a request is received. If the
    /// alarm fires, it indicates that the service has been idle for the
    /// configured timeout and should shut down.
    shutdown_alarm: Pin<Box<Sleep>>,
    /// The initial timeout before the service shuts down if no requests are received.
    pub start_timeout: Duration,
    /// The idle timeout before the service shuts down.
    ///
    /// This timeout is reset every time a request is received.
    pub idle_timeout: Duration,
    /// Flag to signal the server to shut down.
    pub(crate) shutdown: bool,
}

/// `DeviceInfo` holds the state of a single simulated device, including its
/// configuration and the set of chips that belong to it.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// The unique identifier for the device.
    pub id: DeviceId,
    /// A GUID for the device, typically provided by the packet streamer.
    pub guid: Option<String>,
    /// The set of chips that belong to this device.
    pub chips: HashSet<ChipId>,
    /// Device configuration.
    pub device_config: DeviceConfig,
}

/// Holds information about a chip, used in the chip_to_device_map.
#[derive(Debug, Clone)]
pub struct ChipInfo {
    pub kind: NetworkKind,
    pub device_id: DeviceId,
    pub name: String,
    pub manufacturer: String,
    pub product_name: String,
}

impl Server {
    /// Creates a new `Server` and a corresponding `DeviceClient` with default timeouts.
    pub fn new(no_shutdown: bool) -> (Self, DeviceClient) {
        if no_shutdown {
            Self::new_with_timeouts(MAX_TIMEOUT, MAX_TIMEOUT)
        } else {
            Self::new_with_timeouts(DEFAULT_START_TIMEOUT, DEFAULT_IDLE_TIMEOUT)
        }
    }

    /// Creates a new `Server` and a corresponding `DeviceClient` with custom timeouts.
    pub fn new_with_timeouts(
        start_timeout: Duration,
        idle_timeout: Duration,
    ) -> (Self, DeviceClient) {
        let (command_tx, request_rx) = mpsc::channel(10);

        let server = Server {
            chip_clients: HashMap::new(),
            next_chip_id: AtomicU32::new(0),
            next_device_id: AtomicU32::new(0),
            request_rx,
            devices_by_id: HashMap::new(),
            device_ids_by_guid: HashMap::new(),
            chip_info_map: HashMap::new(),
            shutdown_alarm: Box::pin(time::sleep_until(Instant::now())),
            start_timeout,
            idle_timeout,
            shutdown: false,
        };
        (server, DeviceClient::new(command_tx))
    }

    /// Runs the device service's main loop.
    ///
    /// The server will listen for incoming requests and handle them accordingly.
    /// It will shut down if it remains idle for the configured timeout.
    pub async fn run(mut self, chip_clients: HashMap<NetworkKind, ChipClient>) {
        self.chip_clients = chip_clients;
        self.set_alarm(self.start_timeout);
        while !self.shutdown {
            tokio::select! {
                Some(cmd) = self.request_rx.recv() => {
                    self.handle_command(cmd).await;
                }
                _ = &mut self.shutdown_alarm => break,
            }
        }
        info!("Device server is shutting down");
        for (kind, client) in self.chip_clients.iter() {
            if let Err(e) = client.shutdown().await {
                log::error!("Failed to send shutdown to {:?} chip service: {}", kind, e);
            }
        }
        // TODO: We might need to wait for chip services to confirm shutdown
        // if DeviceService needs to ensure they are down before it fully exits.
        info!("Device server has shut down");
    }

    fn set_alarm(&mut self, duration: Duration) {
        let new_deadline = Instant::now() + duration;
        self.shutdown_alarm.as_mut().reset(new_deadline);
    }

    pub fn start_idle_alarm(&mut self) {
        self.set_alarm(self.idle_timeout);
    }

    pub fn stop_idle_alarm(&mut self) {
        // Effectively disable the shutdown alarm by setting a very large duration.
        self.set_alarm(MAX_TIMEOUT);
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

    /// Returns a mutable reference to the `DeviceInfo` for the given `DeviceId`.
    pub(crate) fn get_device_info(
        &mut self,
        id: &DeviceId,
    ) -> Result<&mut DeviceInfo, DeviceError> {
        self.devices_by_id
            .get_mut(id)
            .ok_or_else(|| DeviceError::Internal(format!("Device {id}'s info not found")))
    }

    pub(crate) fn get_chip_client_by_id(
        &self,
        chip_id: ChipId,
    ) -> Result<&ChipClient, DeviceError> {
        let chip_info = self
            .chip_info_map
            .get(&chip_id)
            .ok_or(DeviceError::Internal(format!("Chip {chip_id} not found")))?;
        self.get_chip_client(chip_info.kind)
    }

    /// Retrieves the appropriate `ChipClient` for the given `NetworkKind`.
    pub(crate) fn get_chip_client(&self, kind: NetworkKind) -> Result<&ChipClient, DeviceError> {
        self.chip_clients.get(&kind).ok_or_else(|| {
            DeviceError::Internal(format!("ChipClient for {:?} not supported", kind))
        })
    }

    pub(crate) fn add_chip_to_device(
        &mut self,
        device_id: DeviceId,
        chip_id: ChipId,
        chip_kind: NetworkKind,
        name: String,
        manufacturer: String,
        product_name: String,
    ) -> Result<(), DeviceError> {
        let device_info = self.get_device_info(&device_id)?;
        device_info.chips.insert(chip_id);
        self.chip_info_map.insert(
            chip_id,
            ChipInfo { kind: chip_kind, device_id, name, manufacturer, product_name },
        );
        Ok(())
    }

    /// Removes a device and its associated GUID from the server's state.
    pub(crate) fn delete_device(&mut self, device_id: DeviceId) {
        if let Some(device_info) = self.devices_by_id.remove(&device_id) {
            info!("Removed device {:?}", device_id);
            if let Some(guid) = device_info.guid {
                self.device_ids_by_guid.remove(&guid);
            }
        } else {
            log::warn!("Delete failed: Device {device_id} not found");
        }
    }
}
