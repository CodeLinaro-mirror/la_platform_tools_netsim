// Copyright 2023-2025 The Android Open Source Project

use crate::{
    error::ChipError,
    manager::{
        types::{ChipDied, EmulatedChip},
        ChipState,
    },
};
use bytes::Bytes;
use log::{debug, error, info};
use netsim_api::{BluetoothDeviceParams, ChipPatch, PacketStreamerApi};
use rand::Rng;
use rootcanal::{
    bluetooth::Bluetooth,
    controller::{Callbacks as ControllerCallbacks, Id, Idc},
    types::{Address, Phy},
};
use std::ffi::c_int;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};

/// Manages the packet stream for a single virtual device.
pub(crate) struct VirtualDeviceChip {
    chip_id: u32,
}

impl VirtualDeviceChip {
    /// Creates a new `VirtualDeviceChip` and starts its packet processing loop.
    pub fn new(
        rootcanal: Arc<Bluetooth>,
        params: BluetoothDeviceParams,
        packet_streamer: Box<dyn PacketStreamerApi>,
        chip_death_tx: mpsc::Sender<ChipDied>,
    ) -> ((Self, oneshot::Sender<()>), u32) {
        let address = params
            .address
            .parse()
            .unwrap_or_else(|_| Address { address: rand::thread_rng().gen() });
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (hci_tx, hci_rx) = mpsc::channel::<Bytes>(128);

        let callbacks = Box::new(VirtualDeviceCallbacks { hci_tx });
        let chip_id = rootcanal.new_controller(address, callbacks);

        Self::start_packet_processing_loop(
            chip_id,
            rootcanal.clone(),
            packet_streamer,
            hci_rx,
            shutdown_rx,
            chip_death_tx,
        );

        ((Self { chip_id }, shutdown_tx), chip_id)
    }

    /// Spawns the main packet processing loop for the virtual device.
    fn start_packet_processing_loop(
        chip_id: u32,
        rootcanal: Arc<Bluetooth>,
        mut packet_streamer: Box<dyn PacketStreamerApi>,
        mut hci_rx: mpsc::Receiver<Bytes>,
        mut shutdown_rx: oneshot::Receiver<()>,
        chip_death_tx: mpsc::Sender<ChipDied>,
    ) {
        Handle::current().spawn(async move {
            let mut shutdown = false;
            info!("Starting packet stream handler for chip_id: {chip_id}");
            loop {
                tokio::select! {
                    result = packet_streamer.read_packet() => {
                        if !Self::handle_hci_from_streamer(result, chip_id, &rootcanal) {
                            break;
                        }
                    }
                    Some(packet) = hci_rx.recv() => {
                        if !Self::handle_hci_to_streamer(packet, chip_id, &mut packet_streamer).await {
                            break;
                        }
                    }
                    _ = &mut shutdown_rx => {
                        shutdown = true;
                        break;
                    }
                }
            }
            if !shutdown {
                let _ = chip_death_tx.try_send(ChipDied { chip_id });
            }
            debug!("Packet processing done (shudown={shutdown}) for chip_id: {chip_id}");
        });
    }

    /// Handles a packet read from the streamer, returning false if the loop should break.
    fn handle_hci_from_streamer(
        result: Result<Option<Vec<u8>>, netsim_api::Error>,
        chip_id: u32,
        rootcanal: &Arc<Bluetooth>,
    ) -> bool {
        match result {
            Ok(Some(packet)) => {
                if let Err(err) = rootcanal.receive_hci(chip_id, Idc::Cmd, &packet) {
                    error!("Error sending HCI packet to rootcanal: {err}");
                }
                true // Continue loop
            }
            Ok(None) | Err(_) => false, // Break loop on graceful close or any error.
        }
    }

    /// Handles a packet received from rootcanal, returning false if the loop should break.
    async fn handle_hci_to_streamer(
        packet: Bytes,
        chip_id: u32,
        packet_streamer: &mut Box<dyn PacketStreamerApi>,
    ) -> bool {
        if let Err(err) = packet_streamer.write_packet(packet.to_vec()).await {
            error!("Error writing packet for chip_id: {chip_id}: {err}");
            false // Break loop
        } else {
            true // Continue loop
        }
    }
}

pub(crate) struct VirtualDeviceCallbacks {
    hci_tx: mpsc::Sender<Bytes>,
}

impl ControllerCallbacks for VirtualDeviceCallbacks {
    fn send_hci(&self, _source_id: Id, _idc: Idc, hci_packet: &[u8]) {
        let packet = Bytes::copy_from_slice(hci_packet);
        if let Err(e) = self.hci_tx.try_send(packet) {
            error!("Failed to send HCI packet: {e}, dropping.");
        }
    }

    fn on_receive_ll(&self, _sender_id: Id, _packet: &[u8], _phy: Phy, _rssi: i32) {}

    fn invalid_packet_received(
        &self,
        _source_id: Id,
        _reason: c_int,
        _message: &str,
        _data: &[u8],
    ) {
    }
}

impl EmulatedChip for VirtualDeviceChip {
    fn patch_chip(&mut self, _patch: ChipPatch) -> Result<ChipState, ChipError> {
        self.get_chip()
    }

    fn get_chip(&self) -> Result<ChipState, ChipError> {
        Ok(ChipState { id: self.chip_id })
    }
}
