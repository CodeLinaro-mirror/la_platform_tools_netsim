// Copyright 2023-2025 The Android Open Source Project

use crate::types::EmulatedChip;
use crate::utils::ToChipError;
use bytes::Bytes;
use log::error;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{ChipIdentifier, CreateChipParams};
use netsim_proto::model::Chip as ProtoChip;
use rootcanal::{
    bluetooth::Bluetooth,
    controller::{Callbacks as ControllerCallbacks, Id, Idc},
    types::{Address, Phy},
};
use std::ffi::c_int;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

/// A chip that snoops on all Bluetooth traffic.
#[allow(dead_code)]
pub(crate) struct SnifferChip {
    chip_id: ChipIdentifier,
}

struct SnifferCallbacks {
    packet_tx: mpsc::Sender<Bytes>,
}

impl ControllerCallbacks for SnifferCallbacks {
    fn send_hci(&self, _source_id: Id, _idc: Idc, _data: &[u8]) {
        // Sniffers don't have a host, so this is a no-op.
    }

    fn on_receive_ll(&self, _source_id: Id, packet: &[u8], _phy: Phy, _tx_power: i32) {
        if self.packet_tx.try_send(Bytes::copy_from_slice(packet)).is_err() {
            // This is not a critical error. The sniffer is allowed to drop packets
            // if the downstream consumer is not keeping up.
        }
    }

    fn invalid_packet_received(&self, _source_id: Id, reason: c_int, message: &str, _data: &[u8]) {
        error!("Invalid packet received by sniffer: reason={reason}, message={message}");
    }
}

impl SnifferChip {
    pub fn new(rootcanal: &Arc<Bluetooth>, params: CreateChipParams) -> Result<Self, ChipError> {
        let (packet_tx, mut packet_rx) = mpsc::channel(128);
        let callbacks = Box::new(SnifferCallbacks { packet_tx });
        let chip_id = params.id.as_u32();
        rootcanal
            .new_controller(chip_id, Address { address: [0; 6] }, callbacks)
            .to_chip_error()?;
        let mut packet_streamer = params.packet_streamer;
        // Enable scanning on the new controller.
        let scan_params = vec![0x12, 0x20, 7, 0x01, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00];
        rootcanal.receive_hci(chip_id, Idc::Cmd, &scan_params).to_chip_error()?;
        let scan_enable = vec![0x0c, 0x20, 2, 0x01, 0x00];
        rootcanal.receive_hci(chip_id, Idc::Cmd, &scan_enable).to_chip_error()?;

        Handle::current().spawn(async move {
            while let Some(packet) = packet_rx.recv().await {
                if let Err(e) = packet_streamer.write_packet(packet.to_vec()).await {
                    error!("Failed to write packet to sniffer: {e}");
                    break;
                }
            }
        });

        Ok(Self { chip_id: params.id })
    }
}

impl EmulatedChip for SnifferChip {
    fn update_chip(&mut self, _patch: netsim_proto::model::Chip) -> Result<ProtoChip, ChipError> {
        self.get_chip()
    }

    fn get_chip(&self) -> Result<ProtoChip, ChipError> {
        Ok(ProtoChip::default())
    }
}
