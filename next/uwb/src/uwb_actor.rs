// Copyright 2026 The Android Open Source Project
// src/uwb_actor.rs

use bytes::Bytes;
use client::DeviceClient;
use device_api::DeviceId;
use futures::SinkExt;
use log::{debug, error, info};
use netsim_model::chip::{Chip, ChipCreate, ChipId, PacketSink, PacketStream};
use netsim_model::chip_error::ChipError;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::{StreamMap, StreamNotifyClose};

/// The UWB Actor responsible for managing UWB chips and their state.
pub struct UwbActor {
    /// Active chips map
    pub active_chips: HashMap<ChipId, Chip>,
    /// Stream map for incoming packets
    pub streams: StreamMap<ChipId, StreamNotifyClose<PacketStream>>,

    /// A set of tasks for handling packet sinks.
    pub sink_tasks: JoinSet<ChipId>,
    /// Keeps UCI sender channels for each chip.
    /// TODO: Pass senders to pica library after pica is integrated.
    pub uci_senders: HashMap<ChipId, mpsc::Sender<Bytes>>,
    pub device_client: DeviceClient,
}

impl UwbActor {
    pub fn new(device_client: DeviceClient) -> Self {
        UwbActor {
            active_chips: HashMap::new(),
            streams: StreamMap::new(),

            sink_tasks: JoinSet::new(),
            uci_senders: HashMap::new(),
            device_client,
        }
    }

    pub async fn cleanup_chip(
        &mut self,
        chip_id: ChipId,
        reason: &str,
        device_client: &DeviceClient,
    ) {
        log::info!("Cleaning up chip_id: {} due to: {}", chip_id, reason);
        if let Some(chip) = self.active_chips.remove(&chip_id) {
            log::info!("Chip {} removed from active set.", chip_id);
            // Remove from StreamMap
            self.streams.remove(&chip_id);
            log::info!("Stream for chip {} removed.", chip_id);
            // Drop the sender, signalling the sink task to exit.
            self.uci_senders.remove(&chip_id);
            log::info!("UCI sender for chip {} removed.", chip_id);
            // TODO: Remove from pica
            // We rely on the JoinSet to clean up the completed sink task.
            self.send_chip_died_notification(chip_id, chip.device_id, device_client).await;
        } else {
            log::warn!("cleanup_chip called for non-active chip_id: {}", chip_id);
        }
    }

    async fn run_sink_task(
        mut sink: PacketSink,
        mut receiver: mpsc::Receiver<Bytes>,
        id: ChipId,
    ) -> ChipId {
        while let Some(packet) = receiver.recv().await {
            debug!("Sending packet to sink");
            if sink.send(packet).await.is_err() {
                error!("Failed to send packet to sink");
                break;
            }
        }
        info!("Chip {id} sink exited");
        id
    }

    pub fn create_chip(&mut self, mut params: ChipCreate) -> Result<(), ChipError> {
        let chip_id = params.id;
        log::info!("UwbActor: CreateChip received for chip_id: {}", chip_id);
        if self.active_chips.contains_key(&chip_id) {
            log::error!("Chip {} already exists", chip_id);
            return Err(ChipError::ChipExists(chip_id.into()));
        }
        let stream = params
            .packet_stream
            .take()
            .ok_or(ChipError::Internal("Missing PacketStream".into()))?;
        let sink =
            params.packet_sink.take().ok_or(ChipError::Internal("Missing PacketSink".into()))?;
        let (uci_tx, uci_rx) = mpsc::channel(10);
        self.uci_senders.insert(chip_id, uci_tx);
        self.sink_tasks.spawn(async move { Self::run_sink_task(sink, uci_rx, chip_id).await });
        log::info!("UwbActor: Inserting chip {} into active_chips", chip_id);
        let mut chip = Chip::default();
        chip.id = chip_id.0;
        chip.device_id = params.device_id;
        chip.kind = netsim_model::chip::ChipKind::UWB;
        chip.variant = Some(netsim_model::chip::ChipVariant::Uwb);
        chip.name = Some(params.config.name);
        chip.manufacturer = Some(params.config.manufacturer);
        chip.product_name = Some(params.config.product_name);
        self.active_chips.insert(chip_id, chip);
        self.streams.insert(chip_id, StreamNotifyClose::new(stream));
        Ok(())
    }

    async fn send_chip_died_notification(
        &self,
        chip_id: ChipId,
        device_id: DeviceId,
        device_client: &DeviceClient,
    ) {
        log::info!("Sending ChipDied notification for chip_id: {}", chip_id);
        let dc = device_client.clone();
        tokio::spawn(async move {
            let _ = dc.notify_chip_removed(device_id, chip_id).await;
        });
    }
}
