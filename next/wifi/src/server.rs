// Copyright 2024-2025 The Android Open Source Project

use bytes::Bytes;
use client::DeviceClient;
use device_api::DeviceId;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info};
use netsim_model::chip::{Chip, ChipCreate, ChipId, ChipRequest, PacketSink, PacketStream};
use netsim_model::chip_error::ChipError;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::{StreamMap, StreamNotifyClose};

pub struct Server {
    active_chips: HashMap<ChipId, Chip>,
    streams: StreamMap<ChipId, StreamNotifyClose<PacketStream>>,
    /// Receiver for incoming `ChipRequest` commands.
    command_rx: mpsc::Receiver<ChipRequest>,
    /// Client for sending notifications to the DeviceService.
    device_client: DeviceClient,
    /// A set of tasks for handling packet sinks.
    sink_tasks: JoinSet<ChipId>,
    /// Keeps sender channels for each chip.
    senders: HashMap<ChipId, mpsc::Sender<Bytes>>,
}

impl Server {
    pub fn new(device_client: DeviceClient) -> (Self, netsim_model::chip::RadioChipClient) {
        let (command_tx, command_rx) = mpsc::channel(10);
        let server = Server {
            active_chips: HashMap::new(),
            streams: StreamMap::new(),
            command_rx,
            sink_tasks: JoinSet::new(),
            device_client,
            senders: HashMap::new(),
        };
        (server, netsim_model::chip::RadioChipClient::new(command_tx))
    }

    pub async fn run(mut self) {
        let mut shutdown = false;
        while !shutdown {
            tokio::select! {
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            if let Err(e) = self.handle_command(cmd, &mut shutdown).await {
                                log::error!("Error handling message: {:?}", e);
                            }
                        }
                        None => {
                            log::info!("Command channel closed");
                            break;
                        }
                    }
                }
                    Some((chip_id, packet)) = self.streams.next() => {
                        self.handle_stream_data(chip_id, packet).await;
                    }
            }
        }
        log::info!("WifiServer is shutdown");
    }

    async fn handle_stream_data(&mut self, chip_id: ChipId, packet: Option<Bytes>) {
        match packet {
            Some(data) => {
                log::debug!(
                    "Read {} bytes from stream for chip {}: {:?}",
                    data.len(),
                    chip_id,
                    &data
                );
                // TODO: Send to pica.
            }
            None => {
                log::info!("StreamMap indicated closure for chip {}", chip_id);
                let device_id = self.active_chips.get(&chip_id).map(|c| c.device_id);
                if let Some(device_id) = device_id {
                    self.send_chip_died_notification(chip_id, device_id).await;
                }
            }
        }
    }

    async fn handle_command(
        &mut self,
        msg: ChipRequest,
        shutdown: &mut bool,
    ) -> Result<(), ChipError> {
        match msg {
            ChipRequest::Create { params, respond_to } => {
                respond_to.send(self.handle_create(params)).ok();
            }
            ChipRequest::Delete { id, respond_to } => {
                log::info!("WifiServer: DeleteChip received for chip_id: {}", id);
                self.cleanup_chip(id, "delete_request").await;
                let _ = respond_to.send(Ok(()));
            }
            ChipRequest::Read { id, respond_to } => {
                log::info!(
                    "WifiServer: GetChip received for chip_id: {}. Active chips: {:?}",
                    id,
                    self.active_chips
                );
                if !self.active_chips.contains_key(&id) {
                    let _ = respond_to.send(Err(ChipError::ChipNotFound(id)));
                    return Ok(());
                }
                let chip = self.active_chips.get(&id).ok_or(ChipError::ChipNotFound(id))?;
                let _ = respond_to.send(Ok(chip.clone()));
            }
            ChipRequest::Update { id, patch, respond_to } => {
                if let Some(chip) = self.active_chips.get_mut(&id) {
                    if let Some(pos) = patch.position {
                        chip.position = pos;
                    }
                    if let Some(orient) = patch.orientation {
                        chip.orientation = orient;
                    }
                    let _ = respond_to.send(Ok(chip.clone()));
                } else {
                    let _ = respond_to.send(Err(ChipError::ChipNotFound(id)));
                }
                // TODO: Update Wifi service.
            }
            ChipRequest::Shutdown => {
                *shutdown = true;
            }
            _ => {
                log::warn!("Unhandled ChipRequest: {:?}", msg);
            }
        }
        Ok(())
    }

    async fn cleanup_chip(&mut self, chip_id: ChipId, reason: &str) {
        log::info!("Cleaning up chip_id: {} due to: {}", chip_id, reason);
        if let Some(chip) = self.active_chips.remove(&chip_id) {
            log::info!("Chip {} removed from active set.", chip_id);
            // Remove from StreamMap
            self.streams.remove(&chip_id);
            log::info!("Stream for chip {} removed.", chip_id);
            // Drop the sender, signalling the sink task to exit.
            self.senders.remove(&chip_id);
            log::info!("Sender for chip {} removed.", chip_id);
            // TODO: Remove from hostapd

            // We rely on the JoinSet to clean up the completed sink task.
            self.send_chip_died_notification(chip_id, chip.device_id).await;
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

    fn handle_create(&mut self, mut params: ChipCreate) -> Result<(), ChipError> {
        let chip_id = params.id;
        log::info!("WifiServer: CreateChip received for chip_id: {}", chip_id);

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
        self.senders.insert(chip_id, uci_tx);
        self.sink_tasks.spawn(async move { Self::run_sink_task(sink, uci_rx, chip_id).await });

        log::info!("WifiServer: Inserting chip {} into active_chips", chip_id);
        let mut chip = Chip::default();
        chip.id = chip_id.0;
        chip.device_id = params.device_id;
        // TODO: Populate other fields if available in params
        self.active_chips.insert(chip_id, chip);
        self.streams.insert(chip_id, StreamNotifyClose::new(stream));
        Ok(())
    }

    async fn send_chip_died_notification(&self, chip_id: ChipId, device_id: DeviceId) {
        log::info!("Sending ChipDied notification for chip_id: {}", chip_id);
        let dc = self.device_client.clone();
        tokio::spawn(async move {
            let _ = dc.notify_chip_removed(device_id, chip_id).await;
        });
    }
}
