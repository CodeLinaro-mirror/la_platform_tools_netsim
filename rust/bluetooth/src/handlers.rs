// Copyright 2023-2025 The Android Open Source Project

use crate::{beacon::BeaconChip, sniffer::SnifferChip, virtual_device::VirtualDeviceChip};
use crate::{types::ChipEntry, BluetoothManager};
use log::warn;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{ChipIdentifier, ChipRequest, CreateChipParams};
use netsim_proto::model::Chip as ProtoChip;

impl BluetoothManager {
    pub(super) fn handle_command(&mut self, cmd: ChipRequest) -> bool {
        match cmd {
            ChipRequest::CreateChip { params, respond_to } => {
                let _ = respond_to.send(self.create_chip(params));
            }
            ChipRequest::UpdateChip { id, chip, respond_to } => {
                let _ = respond_to.send(self.update_chip(id, chip));
            }
            ChipRequest::GetChip { id, respond_to } => {
                let _ = respond_to.send(self.get_chip(id));
            }
            ChipRequest::DeleteChip { id, respond_to } => {
                let _ = respond_to.send(self.delete_chip(id));
            }
            ChipRequest::GetChipCountForTesting { respond_to } => {
                let _ = respond_to.send(Ok(self.chips.len()));
            }
            ChipRequest::ResetChip { id } => {
                self.reset_chip(id);
            }
            ChipRequest::GetChipStatistics { respond_to } => {
                let _ = respond_to.send(Ok(Vec::new()));
            }
            ChipRequest::Shutdown => {
                return true;
            }
        }
        false
    }

    fn reset_chip(&mut self, _id: ChipIdentifier) {
        warn!("Not implemented");
    }

    // TODO: Make CreateChipParams have enums for different type of technology.
    fn create_chip(&mut self, params: CreateChipParams) -> Result<(), ChipError> {
        let chip_id = params.id.as_u32();
        let chip_entry = if params.ble_beacon.is_some() {
            let chip = BeaconChip::new(&self.rootcanal, params)?;
            ChipEntry { chip: Box::new(chip), shutdown_tx: None }
        } else if params.bt_properties.is_some() {
            let (chip, shutdown_tx) =
                VirtualDeviceChip::new(self.rootcanal.clone(), params, self.chip_death_tx.clone())?;
            ChipEntry { chip: Box::new(chip), shutdown_tx: Some(shutdown_tx) }
        } else {
            let chip = SnifferChip::new(&self.rootcanal, params)?;
            ChipEntry { chip: Box::new(chip), shutdown_tx: None }
        };

        self.chips.insert(chip_id, chip_entry);
        Ok(())
    }

    fn update_chip(&mut self, id: ChipIdentifier, chip: ProtoChip) -> Result<ProtoChip, ChipError> {
        let chip_entry = &mut self.chips.get_mut(&id.0).ok_or(ChipError::ChipNotFound(id))?;
        chip_entry.chip.update_chip(chip)
    }

    fn get_chip(&self, id: ChipIdentifier) -> Result<ProtoChip, ChipError> {
        let chip_entry = &self.chips.get(&id.0).ok_or(ChipError::ChipNotFound(id))?;
        chip_entry.chip.get_chip()
    }

    fn delete_chip(&mut self, id: ChipIdentifier) -> Result<(), ChipError> {
        if let Some(chip_entry) = self.chips.remove(&id.0) {
            if let Some(tx) = chip_entry.shutdown_tx {
                let _ = tx.send(());
            }
        } else {
            return Err(ChipError::ChipNotFound(id));
        }
        let _ = self.rootcanal.remove_controller(id.0);
        Ok(())
    }
}
