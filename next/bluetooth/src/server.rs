// Copyright 2023-2025 The Android Open Source Project

//! This module provides the Bluetooth `Server` which is the central component for managing Bluetooth
//! simulation.
//!
// To use the `Server`:
// 1. Create a new instance using `Server::new()` which returns the `Server` and a `ChipClient`.
// 2. Spawn the `Server::run()` method into a Tokio task to start its event loop.
//
// ```no_run
// use tokio;
// use netsim_model::device::DeviceRequest;
// use client::DeviceClient;
// use tokio::sync::mpsc;
//
// #[tokio::main]
// async fn main() {
//     let (device_tx, _device_rx) = mpsc::channel::<DeviceRequest>(10);
//     let device_client = DeviceClient::new(device_tx);
//     let (server, client) = bluetooth::Server::new(device_client);
//     tokio::spawn(async move {
//         server.run().await;
//     });
//     // Use the client to interact with the server.
// }
// ```
//
// Features:
// * **Actor-Based State Management:** Implements the actor model, with the `Server` as a central
//   actor that serializes all operations to safely manage the state of multiple Bluetooth
//   chips (Device, Beacon, and Sniffer modes).
// * **HCI Stream/Sink Bridging:** For each chip, bridges a `PacketStream` (for incoming HCI
//   commands) and a `PacketSink` (for outgoing HCI events), routing packets between the host
//   and the `rootcanal` simulation.
// * **Rootcanal Integration:** Simulates the Bluetooth controller logic using `rootcanal`.
//
// Future Features:
// * **RSSI Management:** Manage Received Signal Strength Indication (RSSI) based on chip location.
// * **Link Layer Capture:** Sniffer functionality to convert Rootcanal LL packets to standard Bluetooth LL packets.
// * **HCI-based Beacon:** Implement Beacon functionality via HCI commands, allowing common Android-like advertisement parameters.
use crate::ranging;
use crate::utils::ToChipError;
use bytes::Bytes;
use client::DeviceClient;
use log::{debug, error, info};
use netsim_model::chip::{Chip, ChipClient, ChipId, ChipRequest, PacketStream};
use netsim_model::chip_error::ChipError;
use netsim_model::device::DeviceId;
use rootcanal::{Callbacks as RootcanalCallbacks, Phy, Rootcanal};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::{JoinError, JoinSet};
use tokio::time::{interval, Duration};
use tokio_stream::{StreamExt, StreamMap, StreamNotifyClose};

/// Represents a simulated Bluetooth chip.
#[derive(Debug, Clone)]
pub struct BluetoothChip {
    /// The underlying chip state.
    pub chip: Chip,
    /// The ID of the device this chip belongs to.
    pub device_id: DeviceId,
}

type ChipMap = Arc<Mutex<HashMap<ChipId, BluetoothChip>>>;

/// The `Server` is the central component of the Bluetooth simulation.
///
/// It is responsible for:
/// - Managing the lifecycle of all simulated Bluetooth chips.
/// - Handling commands to create, patch, get, and delete chips.
/// - Running the main event loop that drives the simulation.
/// - Interacting with the `rootcanal` Bluetooth emulator.
pub struct Server {
    // TODO: reduce visibility of fields
    /// The Rootcanal emulator instance.
    pub(crate) rootcanal: Arc<Rootcanal>,
    /// A map of all active chips, keyed by chip ID.
    pub(crate) chips: ChipMap,
    /// Receiver for incoming `ChipRequest` commands.
    command_rx: mpsc::Receiver<ChipRequest>,
    /// A map of all active packet streams, keyed by chip ID.
    pub(crate) streams: StreamMap<ChipId, StreamNotifyClose<PacketStream>>,
    /// A set of tasks for handling packet sinks.
    pub(crate) sink_tasks: JoinSet<ChipId>,
    /// Client for sending notifications to the DeviceService.
    pub(crate) device_client: DeviceClient,
}

/// Implementation of `RootcanalCallbacks` for the Bluetooth `Server`.
/// We need to wrap this in a Mutex even though Rootcanal runs on
/// the same thread because RootcanalCallbacks is not sync.
struct RootcanalCallbacksImpl {
    chips: ChipMap,
}

impl RootcanalCallbacks for RootcanalCallbacksImpl {
    fn on_send_ll(
        &self,
        source_id: u32,
        destination_id: u32,
        _packet: &[u8],
        _phy: Phy,
        tx_power: i32,
    ) -> Option<i32> {
        let src_id = source_id.into();
        let dst_id = destination_id.into();
        let chips = self.chips.lock().unwrap();
        let src_chip = chips.get(&src_id);
        let dst_chip = chips.get(&dst_id);

        if let (Some(src), Some(dst)) = (src_chip, dst_chip) {
            let dist = ranging::distance(&src.chip.position, &dst.chip.position);
            let rssi = ranging::distance_to_rssi(tx_power as i8, dist);
            Some(rssi as i32)
        } else {
            // If one of the chips is missing, default to tx_power.
            // This can happen during startup/shutdown or if a chip is not yet fully registered.
            Some(tx_power)
        }
    }
}

impl Server {
    /// Creates a new `Server` and returns a tuple containing the
    /// server and a `ChipClient` for sending commands to it.
    ///
    /// # Arguments
    ///
    /// * `device_client`: A `DeviceClient` used to send notifications to the DeviceService.
    pub fn new(device_client: DeviceClient) -> (Self, ChipClient) {
        let (command_tx, command_rx) = mpsc::channel(10);
        let chips = Arc::new(Mutex::new(HashMap::new()));
        let server = Server {
            rootcanal: Rootcanal::new(Box::new(RootcanalCallbacksImpl { chips: chips.clone() })),
            chips: chips.clone(),
            command_rx,
            streams: StreamMap::new(),
            sink_tasks: JoinSet::new(),
            device_client,
        };
        (server, ChipClient::new(command_tx))
    }

    /// Processes the next packet from the a stream in `self.streams`.
    ///
    /// This function is called when a packet is received from one of the active
    /// packet streams. It forwards the packet to the `rootcanal` emulator.
    /// If the stream is closed, it removes the chip.
    fn streams_next(&mut self, id: ChipId, val: Option<Bytes>) {
        match val {
            Some(packet) => {
                if packet.is_empty() {
                    error!("Received empty HCI packet from stream for chip {id}");
                    return;
                }
                self.rootcanal.receive_hci(id.into(), packet).expect("Receive HCI error")
            }
            None => {
                if let Err(e) = self.remove_chip(id, "stream_closed") {
                    error!("Failed to remove chip {id} after stream closure: {e}");
                }
            }
        }
    }

    /// Processes the result of a completed sink task.
    ///
    /// This function is called when a task in `self.sink_tasks` finishes.
    /// It removes the chip associated with the completed sink task.
    fn join_next(&mut self, res: Result<ChipId, JoinError>) {
        match res {
            Ok(id) => {
                if let Err(e) = self.remove_chip(id, "sink") {
                    error!("Failed to remove chip {id} after sink task exited: {e}");
                }
            }
            Err(e) => error!("Sink task: {e}"),
        }
    }

    /// Runs the main event loop of the `Server`.
    ///
    /// This function continuously processes incoming commands and events.
    pub async fn run(mut self) {
        let mut tick_interval = interval(Duration::from_millis(10));
        let mut shutdown = false;
        while !shutdown {
            tokio::select! {
                _ = tick_interval.tick() => self.rootcanal.tick(),
                Some(cmd) = self.command_rx.recv() =>
                     self.handle_command(cmd, &mut shutdown).await,
                Some((id, val)) = self.streams.next() => self.streams_next(id, val),
                Some(res) = self.sink_tasks.join_next() => self.join_next(res),
            }
        }
        info!("Bluetooth is shutdown");
    }

    /// Removes a chip from the simulation.
    ///
    /// # Arguments
    ///
    /// * `id`: The ID of the chip to remove.
    /// * `res`: A string indicating the reason for removal.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the chip was removed, `Ok(false)` if the chip
    /// was not found, or an error if removal failed.
    // Return false if the chip doesn't exist.
    pub(crate) fn remove_chip(&mut self, id: ChipId, res: &str) -> Result<bool, ChipError> {
        debug!("Removing chip {id} by {res}");
        if let Some(chip) = self.chips.lock().unwrap().remove(&id) {
            // A chip might not have a stream (e.g. Beacon)
            self.streams.remove(&id);
            self.rootcanal.remove_controller(id.into()).to_chip_error()?;
            debug!("Removed chip {id} by {res}");
            // Notify DeviceServer asynchronously.
            let dc = self.device_client.clone();
            tokio::spawn(async move {
                let _ = dc.notify_chip_removed(chip.device_id, id).await;
            });
            Ok(true)
        } else {
            debug!("Chip already deleted");
            Ok(false)
        }
    }
}
