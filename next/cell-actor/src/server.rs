// Copyright 2024-2025 The Android Open Source Project // touch
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use device_actor::DeviceClient;
use device_api::DeviceId;
use futures::{SinkExt, StreamExt};
use modem_rs::modem_network::{ModemCallbacks, ModemNetworkInterface};
use netsim_model::{
    chip::{ChipId, ChipRequest, PacketSink, PacketStream},
    chip_error::ChipError as NetsimChipError,
};
use tokio::{
    sync::mpsc,
    task::{JoinError, JoinSet},
};
use tokio_stream::{StreamMap, StreamNotifyClose};
use tracing::{debug, error, info, warn};

use crate::error::CellError;

pub struct CellRunner {
    receiver: mpsc::Receiver<ChipRequest>,
}

impl CellRunner {
    pub fn new(receiver: mpsc::Receiver<ChipRequest>) -> Self {
        Self { receiver }
    }

    pub async fn run(mut self, mut server: CellServer) {
        info!("CellServer started");
        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = server.handle_message(msg).await {
                                error!("Error handling message: {:?}", e);
                            }
                        }
                        None => {
                            info!("CellServer channels closed, stopping.");
                            break;
                        }
                    }
                }
                Some((chip_id, packet)) = server.streams.next() => {
                    server.handle_stream_data(chip_id, packet).await;
                }
                Some(res) = server.sink_tasks.join_next() => {
                    server.handle_join_result(res).await;
                }
            }
        }
        info!("CellServer stopped");
    }
}

pub struct CellServer {
    device_client: DeviceClient,
    controller: Arc<dyn ModemNetworkInterface>,
    active_chips: HashMap<ChipId, DeviceId>,
    senders: HashMap<ChipId, mpsc::Sender<Bytes>>,
    streams: StreamMap<ChipId, StreamNotifyClose<PacketStream>>,
    sink_tasks: JoinSet<ChipId>,
    // Keep callbacks alive
    callbacks: HashMap<ChipId, Arc<CellModemCallbacks>>,
}

struct CellModemCallbacks {
    chip_id: ChipId,
    tx: mpsc::Sender<Bytes>,
}

impl ModemCallbacks for CellModemCallbacks {
    fn on_data_received(&self, data: Bytes) {
        debug!("Callback on_data_received for chip {}: {:?}", self.chip_id, data);
        if let Err(e) = self.tx.try_send(data) {
            error!("Failed to send data to sink task for chip {}: {}, dropping.", self.chip_id, e);
        }
    }

    fn on_event(&self, event: String) {
        info!("Controller event for chip {}: {}", self.chip_id, event);
    }
}

async fn run_sink_task(
    mut sink: PacketSink,
    mut receiver: mpsc::Receiver<Bytes>,
    id: ChipId,
) -> ChipId {
    while let Some(packet) = receiver.recv().await {
        debug!("Sending packet to sink for chip {}", id);
        if sink.send(packet).await.is_err() {
            error!("Failed to send packet to sink for chip {}", id);
            break;
        }
    }
    info!("Chip {} sink task exited", id);
    id
}

impl CellServer {
    pub fn new(device_client: DeviceClient, controller: Arc<dyn ModemNetworkInterface>) -> Self {
        CellServer {
            device_client,
            controller,
            active_chips: HashMap::new(),
            senders: HashMap::new(),
            streams: StreamMap::new(),
            sink_tasks: JoinSet::new(),
            callbacks: HashMap::new(),
        }
    }

    async fn handle_stream_data(&mut self, chip_id: ChipId, packet: Option<Bytes>) {
        match packet {
            Some(data) => {
                if data.is_empty() {
                    info!("Stream closed for chip {}", chip_id);
                    self.cleanup_chip(chip_id, "stream_closed").await;
                    return;
                }
                debug!("Read {} bytes from stream for chip {}: {:?}", data.len(), chip_id, &data);
                if let Err(e) = self.controller.send_data(chip_id, &data) {
                    error!("Failed to send data to controller for chip {}: {:?}", chip_id, e);
                }
            }
            None => {
                info!("StreamMap indicated closure for chip {}", chip_id);
                let device_id = self.active_chips.get(&chip_id).cloned();
                if let Some(device_id) = device_id {
                    self.send_chip_died_notification(chip_id, device_id).await;
                }
            }
        }
    }

    async fn handle_join_result(&mut self, res: Result<ChipId, JoinError>) {
        match res {
            Ok(id) => {
                info!("Sink task for chip {} finished", id);
                self.cleanup_chip(id, "sink_task_exited").await;
            }
            Err(e) => error!("Sink task join error: {}", e),
        }
    }

    async fn handle_message(&mut self, msg: ChipRequest) -> Result<(), CellError> {
        match msg {
            ChipRequest::Create { mut params, respond_to } => {
                let chip_id = params.id;
                info!("CellServer: CreateChip received for chip_id: {}", chip_id);

                let stream = params.packet_stream.take().ok_or(CellError::MissingStreamSink)?;
                let sink = params.packet_sink.take().ok_or(CellError::MissingStreamSink)?;
                let device_id = params.device_id;

                if self.active_chips.contains_key(&chip_id) {
                    let err_msg = format!("Chip {} already exists", chip_id);
                    error!("{}", err_msg);
                    let _ = respond_to.send(Err(NetsimChipError::Internal(Box::from(err_msg))));
                    return Ok(());
                }

                let (tx, rx) = mpsc::channel(100); // Channel for on_data_received

                self.sink_tasks.spawn(run_sink_task(sink, rx, chip_id));
                self.senders.insert(chip_id, tx.clone());

                let callbacks = Arc::new(CellModemCallbacks { chip_id, tx });
                if let Err(e) = self.controller.add_modem(chip_id, callbacks.clone()) {
                    error!("Failed to add modem to controller: {:?}", e);
                    let _ = respond_to.send(Err(NetsimChipError::Internal(Box::from(format!(
                        "Controller error: {:?}",
                        e
                    )))));
                    return Ok(());
                }
                self.callbacks.insert(chip_id, callbacks);

                info!("CellServer: Inserting chip {} into active_chips", chip_id);
                self.active_chips.insert(chip_id, device_id);
                self.streams.insert(chip_id, StreamNotifyClose::new(stream));

                let _ = respond_to.send(Ok(()));
            }
            ChipRequest::Delete { id, respond_to } => {
                info!("CellServer: DeleteChip received for chip_id: {}", id);
                self.cleanup_chip(id, "delete_request").await;
                let _ = respond_to.send(Ok(()));
            }
            ChipRequest::Read { id, respond_to } => {
                info!(
                    "CellServer: GetChip received for chip_id: {}. Active chips: {:?}",
                    id, self.active_chips
                );
                if !self.active_chips.contains_key(&id) {
                    let _ = respond_to.send(Err(NetsimChipError::ChipNotFound(id)));
                    return Ok(());
                }

                match self.controller.get_modem_info(id) {
                    Ok(chip) => {
                        let _ = respond_to.send(Ok(chip));
                    }
                    Err(e) => {
                        error!("Failed to get modem info for chip {}: {:?}", id, e);
                        let _ = respond_to.send(Err(NetsimChipError::Internal(Box::from(
                            format!("Controller error: {:?}", e),
                        ))));
                    }
                }
            }
            ChipRequest::GetStatistics { respond_to } => {
                let _ = respond_to.send(Ok(Box::new([])));
            }
            _ => {
                warn!("Unhandled ChipRequest: {:?}", msg);
            }
        }
        Ok(())
    }

    async fn cleanup_chip(&mut self, chip_id: ChipId, reason: &str) {
        info!("Cleaning up chip_id: {} due to: {}", chip_id, reason);
        if let Some(device_id) = self.active_chips.remove(&chip_id) {
            info!("Chip {} removed from active set.", chip_id);
            if let Err(e) = self.controller.remove_modem(chip_id) {
                error!("Failed to remove modem from controller: {:?}", e);
            }
            self.callbacks.remove(&chip_id);
            self.senders.remove(&chip_id);
            // Remove from StreamMap
            self.streams.remove(&chip_id);
            info!("Stream for chip {} removed.", chip_id);
            self.send_chip_died_notification(chip_id, device_id).await;
        } else {
            warn!("cleanup_chip called for non-active chip_id: {}", chip_id);
        }
    }

    async fn send_chip_died_notification(&self, chip_id: ChipId, device_id: DeviceId) {
        info!("Sending ChipDied notification for chip_id: {}", chip_id);
        let dc = self.device_client.clone();
        tokio::spawn(async move {
            let _ = dc.notify_chip_removed(device_id, chip_id).await;
        });
    }
}
