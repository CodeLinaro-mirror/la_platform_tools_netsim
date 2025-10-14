// Copyright 2023-2025 The Android Open Source Project

use crate::{
    beacon::BeaconChip,
    device::DeviceChip,
    sniffer::SnifferChip,
    types::{ChipEntry, EmulatedChip},
    BluetoothManager,
};
use log::warn;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{
    BluetoothMode, ChipIdentifier, ChipRequest, CreateChipParams, NetworkParams,
};
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

    fn create_chip(&mut self, create_params: CreateChipParams) -> Result<(), ChipError> {
        let chip_id = create_params.id;
        let bluetooth_mode = match create_params.network_params {
            NetworkParams::Bluetooth(bt_params) => bt_params,
            _ => {
                return Err(ChipError::InvalidArguments("Unsupported chip kind".to_string()));
            }
        };

        let (chip, shutdown_tx) = match &bluetooth_mode {
            BluetoothMode::Beacon(params) => {
                let (chip, shutdown_tx) =
                    BeaconChip::new(&self.rootcanal, chip_id.clone(), params)?;
                (Box::new(chip) as Box<dyn EmulatedChip>, shutdown_tx)
            }
            BluetoothMode::Device(params) => {
                let (chip, shutdown_tx) = DeviceChip::new(
                    self.rootcanal.clone(),
                    chip_id.clone(),
                    create_params.packet_streamer,
                    params,
                    self.chip_death_tx.clone(),
                )?;
                (Box::new(chip) as Box<dyn EmulatedChip>, shutdown_tx)
            }
            BluetoothMode::Sniffer(params) => {
                let (chip, shutdown_tx) = SnifferChip::new(
                    &self.rootcanal,
                    chip_id.clone(),
                    params,
                    create_params.packet_streamer,
                )?;
                (Box::new(chip) as Box<dyn EmulatedChip>, shutdown_tx)
            }
        };

        self.chips.insert(chip_id.as_u32(), ChipEntry { chip, shutdown_tx, bluetooth_mode });
        Ok(())
    }

    fn update_chip(&mut self, id: ChipIdentifier, chip: ProtoChip) -> Result<ProtoChip, ChipError> {
        let chip_entry =
            &mut self.chips.get_mut(&id.as_u32()).ok_or(ChipError::ChipNotFound(id))?;
        chip_entry.chip.update_chip(chip)
    }

    fn get_chip(&self, id: ChipIdentifier) -> Result<ProtoChip, ChipError> {
        let chip_entry = &self.chips.get(&id.as_u32()).ok_or(ChipError::ChipNotFound(id))?;
        chip_entry.chip.get_chip()
    }

    fn delete_chip(&mut self, id: ChipIdentifier) -> Result<(), ChipError> {
        if let Some(chip_entry) = self.chips.remove(&id.as_u32()) {
            if let Some(tx) = chip_entry.shutdown_tx {
                let _ = tx.send(());
            }
        } else {
            return Err(ChipError::ChipNotFound(id));
        }
        let _ = self.rootcanal.remove_controller(id.as_u32());
        Ok(())
    }
}
