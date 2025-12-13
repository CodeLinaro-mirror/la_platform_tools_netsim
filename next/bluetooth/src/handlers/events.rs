// Copyright 2025 The Android Open Source Project

use crate::context::BluetoothContext;
use crate::error::BluetoothError;
use actor_framework::Runtime;
use bytes::Bytes;
use netsim_model::chip::ChipId;
use rootcanal::controller::{Callbacks as ControllerCallbacks, Id};
use rootcanal::types::Phy;
use std::ffi::c_int;
use tokio::sync::mpsc;

pub struct HciCallbacks {
    pub id: ChipId,
    pub hci_tx: Option<mpsc::Sender<Bytes>>,
    pub ll_tx: Option<mpsc::Sender<Bytes>>,
}

impl ControllerCallbacks for HciCallbacks {
    fn send_hci(&self, _source_id: Id, h4_packet: Bytes) {
        if let Some(hci_tx) = self.hci_tx.as_ref() {
            if let Err(e) = hci_tx.try_send(h4_packet) {
                log::error!("Failed to send HCI packet: {e}, dropping.");
            }
        }
    }

    fn on_receive_ll(&self, _source_id: Id, packet: &[u8], _phy: Phy, _tx_power: i32) {
        log::debug!("[{}] Received LL packet", self.id);
        if let Some(ll_tx) = self.ll_tx.as_ref() {
            let packet = Bytes::copy_from_slice(packet);
            if let Err(e) = ll_tx.try_send(packet) {
                log::error!("Failed to send LL packet: {e}, dropping.");
            }
        }
    }

    fn invalid_packet_received(&self, _source_id: Id, reason: c_int, message: &str, _data: &[u8]) {
        log::error!(
            "[chip-{}] Invalid packet received by device: reason={reason}, message={message}",
            self.id
        );
    }
}

pub async fn on_stream(
    context: &mut BluetoothContext,
    id: usize,
    message: Bytes,
    _runtime: &mut impl Runtime,
) {
    if message.is_empty() {
        log::error!("Received empty HCI packet from stream for chip {id}");
        return;
    }
    if let Err(e) = context.rootcanal.receive_hci(id as u32, message) {
        log::error!("Receive HCI error for chip {id}: {e}");
    }
}
