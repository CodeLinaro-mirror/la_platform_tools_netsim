// Copyright 2023-2025 The Android Open Source Project

//! This module implements the command handlers for the Bluetooth `Server`.
//!
//! It provides the `handle_command` method and related functions for processing
//! `ChipRequest` messages, managing the lifecycle of Bluetooth chips, and
//! interacting with the `rootcanal` Bluetooth emulator.

use crate::server::ChipEntry;
use crate::utils::ToChipError;
use crate::Server;
use bytes::Bytes;
use futures::SinkExt;
use log::{debug, error, info, warn};
use netsim_api::{
    chip_error::ChipError,
    chips::{BluetoothMode, ChipId, ChipRequest, CreateParams, NetworkParams, PacketSink},
};
use netsim_proto::model::Chip as ProtoChip;
use rootcanal::{
    controller::{Callbacks as ControllerCallbacks, Id, Idc},
    types::{Address, Phy},
};
use std::ffi::c_int;
use tokio::sync::mpsc;
use tokio_stream::StreamNotifyClose;

pub(crate) struct HciCallbacks {
    id: ChipId,
    hci_tx: Option<mpsc::Sender<Bytes>>,
    ll_tx: Option<mpsc::Sender<Bytes>>,
}

impl ControllerCallbacks for HciCallbacks {
    fn send_hci(&self, _source_id: Id, _idc: Idc, hci_packet: &[u8]) {
        let packet = Bytes::copy_from_slice(hci_packet);
        if let Some(hci_tx) = self.hci_tx.as_ref() {
            if let Err(e) = hci_tx.try_send(packet) {
                error!("Failed to send HCI packet: {e}, dropping.");
            }
        }
    }

    fn on_receive_ll(&self, _source_id: Id, packet: &[u8], _phy: Phy, _tx_power: i32) {
        debug!("[{}] Received LL packet", self.id);
        if let Some(ll_tx) = self.ll_tx.as_ref() {
            let packet = Bytes::copy_from_slice(packet);
            if let Err(e) = ll_tx.try_send(packet) {
                error!("Failed to send LL packet: {e}, dropping.");
            }
        }
    }

    fn invalid_packet_received(&self, _source_id: Id, reason: c_int, message: &str, _data: &[u8]) {
        error!(
            "[chip-{}] Invalid packet received by device: reason={reason}, message={message}",
            self.id
        );
    }
}

impl Server {
    pub(super) async fn handle_command(&mut self, cmd: ChipRequest, shutdown: &mut bool) {
        match cmd {
            ChipRequest::Create { params: create_params, respond_to } => {
                respond_to.send(self.create_chip(create_params)).ok();
            }
            ChipRequest::Read { id, respond_to } => {
                respond_to.send(self.get_chip(id)).ok();
            }
            ChipRequest::Update { id, chip, respond_to } => {
                respond_to.send(self.update_chip(id, chip)).ok();
            }
            ChipRequest::Delete { id, respond_to } => {
                respond_to.send(self.delete_chip(id)).ok();
            }
            ChipRequest::Reset { id } => {
                self.reset_chip(id);
            }
            ChipRequest::GetStatistics { respond_to } => {
                respond_to.send(Ok(Vec::new())).ok();
            }
            ChipRequest::GetCountForTesting { respond_to } => {
                respond_to.send(Ok(self.chips.len())).ok();
            }
            ChipRequest::Shutdown => {
                *shutdown = true;
            }
        }
    }

    fn reset_chip(&mut self, _id: ChipId) {
        warn!("Not implemented");
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

    fn create_chip(&mut self, mut create_params: CreateParams) -> Result<(), ChipError> {
        let bluetooth_params = match create_params.config.network_params {
            NetworkParams::Bluetooth(params) => params,
            _ => return Err(ChipError::InvalidArguments("Unsupported chip kind".to_string())),
        };

        let id = create_params.id;
        if let Some(stream) = create_params.packet_stream.take() {
            self.streams.insert(id, StreamNotifyClose::new(stream));
        }

        let callback = if let Some(sink) = create_params.packet_sink.take() {
            let (hci_tx, hci_rx) = mpsc::channel(10);
            self.sink_tasks.spawn(async move { Self::run_sink_task(sink, hci_rx, id).await });
            HciCallbacks { id, hci_tx: Some(hci_tx), ll_tx: None }
        } else {
            HciCallbacks { id, hci_tx: None, ll_tx: None }
        };

        let address = bluetooth_params
            .address
            .parse()
            .unwrap_or_else(|_| Address { address: rand::random() });
        let rootcanal = &self.rootcanal;

        rootcanal.new_controller(id.into(), address, Box::new(callback)).to_chip_error()?;

        match &bluetooth_params.mode {
            BluetoothMode::Beacon(params) => crate::beacon::create(rootcanal, id, params)?,
            BluetoothMode::Device(params) => crate::device::create(rootcanal, id, params)?,
            BluetoothMode::Sniffer(params) => crate::sniffer::create(rootcanal, id, params)?,
        }
        self.chips.insert(id, ChipEntry { bluetooth_mode: bluetooth_params.mode });
        Ok(())
    }

    fn update_chip(&mut self, id: ChipId, _chip: ProtoChip) -> Result<ProtoChip, ChipError> {
        let _ = &mut self.chips.get_mut(&id).ok_or(ChipError::ChipNotFound(id))?;
        Ok(ProtoChip::default())
    }

    fn get_chip(&self, id: ChipId) -> Result<ProtoChip, ChipError> {
        let _ = &self.chips.get(&id).ok_or(ChipError::ChipNotFound(id))?;
        Ok(ProtoChip::default())
    }

    fn delete_chip(&mut self, id: ChipId) -> Result<(), ChipError> {
        // TODO: decide if sink task needs to be shutdown
        match self.remove_chip(id, "handler") {
            Ok(true) => Ok(()),
            Ok(false) => Err(ChipError::ChipNotFound(id)),
            Err(e) => Err(e),
        }
    }
}
