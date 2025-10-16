// Copyright 2023-2025 The Android Open Source Project

use crate::utils::ToChipError;
use bytes::Bytes;
use log::{debug, error, info};
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{BluetoothMode, ChipClient, ChipId, ChipRequest, PacketStream};
use rootcanal::{
    bluetooth::{Bluetooth as Rootcanal, Callbacks as RootcanalCallbacks},
    types::Phy,
    Idc,
};
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
/// - Interacting with the `rootcanal` backend.
pub struct Server {
    pub(crate) rootcanal: Arc<Rootcanal>,
    pub(crate) chips: HashMap<ChipId, ChipEntry>,
    pub(crate) command_rx: mpsc::Receiver<ChipRequest>,
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
            Some(packet) => self
                .rootcanal
                .receive_hci(id.as_u32(), Idc::Cmd, &packet)
                .expect("Receive HCI error"),
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
            self.rootcanal.remove_controller(id.as_u32()).to_chip_error()?;
            debug!("Removed chip {id} by {res}");
            Ok(true)
        } else {
            debug!("Chip already deleted");
            Ok(false)
        }
    }
}
