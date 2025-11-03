// src/server.rs

use crate::error::CellError;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use modem_rs::modem_network::{ModemCallbacks, ModemNetworkInterface};
use netsim_api::chip_error::ChipError as NetsimChipError;
use netsim_api::chips::{ChipDiedParams, ChipId, ChipRequest, PacketSink, PacketStream};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::{StreamMap, StreamNotifyClose};

pub struct CellServer {
    device_server_tx: mpsc::Sender<ChipDiedParams>,
    receiver: mpsc::Receiver<ChipRequest>,
    controller_service: Arc<dyn ModemNetworkInterface>,
    active_chips: HashSet<ChipId>,
    streams: StreamMap<ChipId, StreamNotifyClose<PacketStream>>,
    internal_shutdown_tx: mpsc::Sender<ChipId>,
    internal_shutdown_rx: mpsc::Receiver<ChipId>,
    sink_tasks: JoinSet<ChipId>,
}

struct CellModemCallbacks {
    chip_id: ChipId,
    tx: mpsc::Sender<Bytes>,
}

impl ModemCallbacks for CellModemCallbacks {
    fn on_data_received(&self, data: Bytes) {
        log::debug!("Callback on_data_received for chip {}: {:?}", self.chip_id, data);
        if let Err(e) = self.tx.try_send(data) {
            log::error!(
                "Failed to send data to sink task for chip {}: {}, dropping.",
                self.chip_id,
                e
            );
        }
    }

    fn on_event(&self, event: String) {
        log::info!("Controller event for chip {}: {}", self.chip_id, event);
    }
}

async fn run_sink_task(
    mut sink: PacketSink,
    mut receiver: mpsc::Receiver<Bytes>,
    id: ChipId,
) -> ChipId {
    while let Some(packet) = receiver.recv().await {
        log::debug!("Sending packet to sink for chip {}", id);
        if sink.send(packet).await.is_err() {
            log::error!("Failed to send packet to sink for chip {}", id);
            break;
        }
    }
    log::info!("Chip {} sink task exited", id);
    id
}

impl CellServer {
    pub fn new(
        device_server_tx: mpsc::Sender<ChipDiedParams>,
        receiver: mpsc::Receiver<ChipRequest>,
        controller_service: Arc<dyn ModemNetworkInterface>,
    ) -> Self {
        let (internal_shutdown_tx, internal_shutdown_rx) = mpsc::channel(100);
        CellServer {
            device_server_tx,
            receiver,
            controller_service,
            active_chips: HashSet::new(),
            streams: StreamMap::new(),
            internal_shutdown_tx,
            internal_shutdown_rx,
            sink_tasks: JoinSet::new(),
        }
    }

    pub async fn run(mut self) {
        log::info!("CellServer started");
        let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(100));

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    if let Err(e) = self.handle_message(msg).await {
                        log::error!("Error handling message: {:?}", e);
                    }
                }
                Some(chip_id) = self.internal_shutdown_rx.recv() => {
                    log::info!("Received internal shutdown for chip {}", chip_id);
                    self.cleanup_chip(chip_id, "internal_shutdown").await;
                }
                _ = tick_interval.tick() => {
                    self.controller_service.tick();
                }
                Some((chip_id, packet)) = self.streams.next() => {
                    self.handle_stream_data(chip_id, packet).await;
                }
                Some(res) = self.sink_tasks.join_next() => {
                    match res {
                        Ok(id) => {
                            log::info!("Sink task for chip {} finished", id);
                            // Request cleanup for this chip
                            if self.internal_shutdown_tx.send(id).await.is_err() {
                                log::error!("Failed to send internal shutdown for chip {} after sink task exit", id);
                            }
                        }
                        Err(e) => log::error!("Sink task join error: {}", e),
                    }
                }
                else => {
                    log::info!("CellServer channels closed, stopping.");
                    break;
                }
            }
        }
        log::info!("CellServer stopped");
    }

    async fn handle_stream_data(&mut self, chip_id: ChipId, packet: Option<Bytes>) {
        match packet {
            Some(data) => {
                if data.is_empty() {
                    log::info!("Stream closed for chip {}", chip_id);
                    if self.internal_shutdown_tx.send(chip_id).await.is_err() {
                        log::error!("Failed to send internal shutdown for chip {}", chip_id);
                    }
                    return;
                }
                log::debug!(
                    "Read {} bytes from stream for chip {}: {:?}",
                    data.len(),
                    chip_id,
                    &data
                );
                if let Err(e) = self.controller_service.send_data(chip_id, &data) {
                    log::error!("Error sending data to controller {}: {}, cleaning up", chip_id, e);
                    if self.internal_shutdown_tx.send(chip_id).await.is_err() {
                        log::error!("Failed to send internal shutdown for chip {}", chip_id);
                    }
                }
            }
            None => {
                log::info!("StreamMap indicated closure for chip {}", chip_id);
                if self.internal_shutdown_tx.send(chip_id).await.is_err() {
                    log::error!("Failed to send internal shutdown for chip {}", chip_id);
                }
            }
        }
    }

    async fn handle_message(&mut self, msg: ChipRequest) -> Result<(), CellError> {
        match msg {
            ChipRequest::Create { mut params, respond_to } => {
                let chip_id = params.id;
                log::info!("CellServer: CreateChip received for chip_id: {}", chip_id);

                let stream = params.packet_stream.take().ok_or(CellError::MissingStreamSink)?;
                let sink = params.packet_sink.take().ok_or(CellError::MissingStreamSink)?;

                if self.active_chips.contains(&chip_id) {
                    let err_msg = format!("Chip {} already exists", chip_id);
                    log::error!("{}", err_msg);
                    let _ = respond_to.send(Err(NetsimChipError::Internal(err_msg)));
                    return Ok(());
                }

                let (tx, rx) = mpsc::channel(100); // Channel for on_data_received

                self.sink_tasks.spawn(run_sink_task(sink, rx, chip_id));

                let callbacks = Arc::new(CellModemCallbacks { chip_id, tx });

                self.controller_service
                    .add_modem(chip_id, callbacks)
                    .map_err(|e| CellError::ModemError(e.to_string()))?;
                log::info!("CellServer: Inserting chip {} into active_chips", chip_id);
                self.active_chips.insert(chip_id);
                self.streams.insert(chip_id, StreamNotifyClose::new(stream));

                let _ = respond_to.send(Ok(()));
            }
            ChipRequest::Delete { id, respond_to } => {
                log::info!("CellServer: DeleteChip received for chip_id: {}", id);
                self.cleanup_chip(id, "delete_request").await;
                let _ = respond_to.send(Ok(()));
            }
            ChipRequest::Read { id, respond_to } => {
                log::info!(
                    "CellServer: GetChip received for chip_id: {}. Active chips: {:?}",
                    id,
                    self.active_chips
                );
                if !self.active_chips.contains(&id) {
                    let _ = respond_to.send(Err(NetsimChipError::ChipNotFound(id)));
                    return Ok(());
                }
                match self.controller_service.get_modem_info(id) {
                    Ok(info) => {
                        let _ = respond_to.send(Ok(info));
                    }
                    Err(e) => {
                        let _ = respond_to.send(Err(NetsimChipError::Internal(e.to_string())));
                    }
                }
            }
            _ => {
                log::warn!("Unhandled ChipRequest: {:?}", msg);
            }
        }
        Ok(())
    }

    async fn cleanup_chip(&mut self, chip_id: ChipId, reason: &str) {
        log::info!("Cleaning up chip_id: {} due to: {}", chip_id, reason);
        if self.active_chips.remove(&chip_id) {
            log::info!("Chip {} removed from active set.", chip_id);
            if let Err(e) = self.controller_service.remove_modem(chip_id) {
                log::error!("Error removing controller {}: {}", chip_id, e);
            }
            // Remove from StreamMap
            self.streams.remove(&chip_id);
            log::info!("Stream for chip {} removed.", chip_id);
            // SINK_TASK: The sink task will exit when the sender is dropped.
            // We rely on the JoinSet to clean up.
            self.send_chip_died_notification(chip_id).await;
        } else {
            log::warn!("cleanup_chip called for non-active chip_id: {}", chip_id);
        }
    }

    async fn send_chip_died_notification(&self, chip_id: ChipId) {
        log::info!("Sending ChipDied notification for chip_id: {}", chip_id);
        let msg = ChipDiedParams { id: chip_id };
        if let Err(e) = self.device_server_tx.send(msg).await {
            log::error!("Failed to send ChipDied notification for chip {}: {}", chip_id, e);
        }
    }
}
