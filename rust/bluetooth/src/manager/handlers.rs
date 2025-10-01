// Copyright 2023-2025 The Android Open Source Project

use super::{
    manager_impl::BluetoothManager,
    types::{BluetoothCommand, ChipEntry, ChipState},
};
use crate::{
    beacon::beacon_impl::BeaconChip, error::ChipError, sniffer::sniffer_impl::SnifferChip,
    virtual_device::virtual_device_impl::VirtualDeviceChip,
};
use netsim_api::{ChipParams, CreateChipParams, DeleteChipParams, GetChipParams, PatchChipParams};

impl BluetoothManager {
    pub(super) fn handle_command(&self, cmd: BluetoothCommand) {
        match cmd {
            BluetoothCommand::CreateChip { params, responder } => {
                let _ = responder.send(self.create_chip(params));
            }
            BluetoothCommand::PatchChip { params, responder } => {
                let _ = responder.send(self.patch_chip(params));
            }
            BluetoothCommand::GetChip { params, responder } => {
                let _ = responder.send(self.get_chip(params));
            }
            BluetoothCommand::DeleteChip { params, responder } => {
                let _ = responder.send(self.delete_chip(params));
            }
        }
    }

    fn create_chip(&self, params: CreateChipParams) -> Result<u32, ChipError> {
        let (chip_id, chip_entry) = match params.chip_params {
            ChipParams::BluetoothDevice(device_params) => {
                let ((chip, shutdown_tx), chip_id) = VirtualDeviceChip::new(
                    self.rootcanal.clone(),
                    device_params,
                    params.packet_streamer,
                    self.chip_death_tx.clone(),
                );
                (chip_id, ChipEntry { chip: Box::new(chip), shutdown_tx: Some(shutdown_tx) })
            }
            ChipParams::BluetoothBeacon(beacon_params) => {
                let (chip, chip_id) = BeaconChip::new(&self.rootcanal, beacon_params)?;
                (chip_id, ChipEntry { chip: Box::new(chip), shutdown_tx: None })
            }
            ChipParams::BluetoothSniffer(_sniffer_params) => {
                let (chip, chip_id) = SnifferChip::new(&self.rootcanal, params.packet_streamer);
                (chip_id, ChipEntry { chip: Box::new(chip), shutdown_tx: None })
            }
        };

        self.chips.lock().unwrap().insert(chip_id, chip_entry);
        Ok(chip_id)
    }

    fn patch_chip(&self, params: PatchChipParams) -> Result<ChipState, ChipError> {
        let mut chips = self.chips.lock().unwrap();
        let chip_entry =
            chips.get_mut(&params.chip_id).ok_or(ChipError::ChipNotFound(params.chip_id))?;
        chip_entry.chip.patch_chip(params.patch)
    }

    fn get_chip(&self, params: GetChipParams) -> Result<ChipState, ChipError> {
        let chips = self.chips.lock().unwrap();
        let chip_entry =
            chips.get(&params.chip_id).ok_or(ChipError::ChipNotFound(params.chip_id))?;
        chip_entry.chip.get_chip()
    }

    fn delete_chip(&self, params: DeleteChipParams) -> Result<(), ChipError> {
        if let Some(chip_entry) = self.chips.lock().unwrap().remove(&params.chip_id) {
            if let Some(tx) = chip_entry.shutdown_tx {
                let _ = tx.send(());
            }
        } else {
            return Err(ChipError::ChipNotFound(params.chip_id));
        }
        let _ = self.rootcanal.remove_controller(params.chip_id);
        Ok(())
    }
}
