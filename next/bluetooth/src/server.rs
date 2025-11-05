// Copyright 2023-2025 The Android Open Source Project

//! This module provides the Bluetooth `Server` which is the central component for managing Bluetooth
//! simulation.
//!
//! To use the `Server`:
//! 1. Create a new instance using `Server::new()` which returns the `Server` and a `ChipClient`.
//! 2. Spawn the `Server::run()` method into a Tokio task to start its event loop.
//!
//! ```no_run
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let (server, client) = bluetooth::Server::new();
//!     tokio::spawn(async move {
//!         server.run().await;
//!     });
//!     // Use the client to interact with the server.
//! }
//! ```
//!
//! Features:
//! * **Actor-Based State Management:** Implements the actor model, with the `Server` as a central
//!   actor that serializes all operations to safely manage the state of multiple Bluetooth
//!   chips (Device, Beacon, and Sniffer modes).
//! * **HCI Stream/Sink Bridging:** For each chip, bridges a `PacketStream` (for incoming HCI
//!   commands) and a `PacketSink` (for outgoing HCI events), routing packets between the host
//!   and the `rootcanal` simulation.
//! * **Rootcanal Integration:** Simulates the Bluetooth controller logic using `rootcanal`.
//!
//! Future Features:
//! * **RSSI Management:** Manage Received Signal Strength Indication (RSSI) based on chip location.
//! * **Link Layer Capture:** Sniffer functionality to convert Rootcanal LL packets to standard Bluetooth LL packets.
//! * **HCI-based Beacon:** Implement Beacon functionality via HCI commands, allowing common Android-like advertisement parameters.
use crate::utils::ToChipError;
use bytes::Bytes;
use log::{debug, error, info};
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{BluetoothMode, ChipClient, ChipId, ChipRequest, PacketStream};
use rootcanal::{Callbacks as RootcanalCallbacks, Idc, Phy, Rootcanal};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::{JoinError, JoinSet};
use tokio::time::{interval, Duration};
use tokio_stream::{StreamExt, StreamMap, StreamNotifyClose};

pub(crate) struct ChipEntry {
    #[allow(dead_code)]
    pub(crate) bluetooth_mode: BluetoothMode,
}

/// The `Server` is the central component of the Bluetooth simulation.
///
/// It is responsible for:
/// - Managing the lifecycle of all simulated Bluetooth chips.
/// - Handling commands to create, patch, get, and delete chips.
/// - Running the main event loop that drives the simulation.
/// - Interacting with the `rootcanal` Bluetooth emulator.
pub struct Server {
    // TODO: reduce visibility of fields
    pub(crate) rootcanal: Arc<Rootcanal>,
    pub(crate) chips: HashMap<ChipId, ChipEntry>,
    command_rx: mpsc::Receiver<ChipRequest>,
    // A map of all active packet streams, keyed by chip ID.
    pub(crate) streams: StreamMap<ChipId, StreamNotifyClose<PacketStream>>,
    pub(crate) sink_tasks: JoinSet<ChipId>,
}

struct RootcanalCallbacksImpl;

impl RootcanalCallbacks for RootcanalCallbacksImpl {
    fn on_send_ll(
        &self,
        _source_id: u32,
        _destination_id: u32,
        _packet: &[u8],
        _phy: Phy,
        tx_power: i32,
    ) -> Option<i32> {
        Some(tx_power)
    }
}

impl Server {
    /// Creates a new `Server` and returns a tuple containing the
    /// server and a channel for sending commands to it.
    pub fn new() -> (Self, ChipClient) {
        let (command_tx, command_rx) = mpsc::channel(10);
        let server = Server {
            rootcanal: Rootcanal::new(Box::new(RootcanalCallbacksImpl {})),
            chips: HashMap::new(),
            command_rx,
            streams: StreamMap::new(),
            sink_tasks: JoinSet::new(),
        };
        (server, ChipClient::new(command_tx))
    }

    fn streams_next(&mut self, id: ChipId, val: Option<Bytes>) {
        match val {
            Some(packet) => {
                self.rootcanal.receive_hci(id.into(), Idc::Cmd, &packet).expect("Receive HCI error")
            }
            None => {
                if let Err(e) = self.remove_chip(id, "stream") {
                    error!("Failed to remove chip {id} after stream closure: {e}");
                }
            }
        }
    }

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

    // Return false if the chip doesn't exist.
    pub(crate) fn remove_chip(&mut self, id: ChipId, res: &str) -> Result<bool, ChipError> {
        debug!("Removing chip {id} by {res}");
        if self.chips.remove(&id).is_some() {
            // A chip might not have a stream (e.g. Beacon)
            self.streams.remove(&id);
            self.rootcanal.remove_controller(id.into()).to_chip_error()?;
            debug!("Removed chip {id} by {res}");
            Ok(true)
        } else {
            debug!("Chip already deleted");
            Ok(false)
        }
    }
}
