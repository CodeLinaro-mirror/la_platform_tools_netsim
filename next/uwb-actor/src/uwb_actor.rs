// Copyright 2026 The Android Open Source Project

use bytes::Bytes;
use client::DeviceClient;
use futures::SinkExt;
use log::{debug, error, info};
use netsim_model::chip::{Chip, ChipId, PacketSink};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// The UWB Actor responsible for managing UWB chips and their state.
pub struct UwbActor {
    /// Map of active chips.
    pub chips: HashMap<ChipId, Chip>,
    /// Senders for forwarding UCI packets to the sink tasks.
    pub uci_senders: HashMap<ChipId, mpsc::Sender<Bytes>>,
    /// Client for interacting with the device actor.
    pub device_client: DeviceClient,
}

impl UwbActor {
    pub fn new(device_client: DeviceClient) -> Self {
        UwbActor { chips: HashMap::new(), uci_senders: HashMap::new(), device_client }
    }
}

/// Runs a task that forwards packets from a channel to a packet sink.
pub async fn run_sink_task(
    mut sink: PacketSink,
    mut receiver: mpsc::Receiver<Bytes>,
    id: ChipId,
) -> ChipId {
    while let Some(packet) = receiver.recv().await {
        debug!("Sending packet to sink for chip {id}");
        if sink.send(packet).await.is_err() {
            error!("Sink for chip {id} is closed.");
            break;
        }
    }
    info!("Chip {id} sink task finished.");
    id
}
