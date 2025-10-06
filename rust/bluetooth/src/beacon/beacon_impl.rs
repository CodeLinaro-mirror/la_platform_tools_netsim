// Copyright 2023-2025 The Android Open Source Project

use crate::{
    error::ChipError,
    manager::{types::EmulatedChip, ChipState},
};
use netsim_api::{BeaconCreationParams, ChipPatch};
use rootcanal::{
    bluetooth::Bluetooth,
    controller::{Callbacks as ControllerCallbacks, Id, Idc},
    types::{Address, Phy},
};
use std::ffi::c_int;
use std::sync::Arc;

/// A single BLE beacon chip.
pub(crate) struct BeaconChip {
    chip_id: u32,
}

pub(crate) struct BeaconControllerCallbacks;

impl ControllerCallbacks for BeaconControllerCallbacks {
    fn send_hci(&self, _source_id: Id, _idc: Idc, _data: &[u8]) {
        // Beacons don't have a host, so this is a no-op.
    }

    fn on_receive_ll(&self, _source_id: Id, _packet: &[u8], _phy: Phy, _tx_power: i32) {
        // Beacons don't need to process incoming packets.
    }

    fn invalid_packet_received(
        &self,
        _source_id: Id,
        _reason: c_int,
        _message: &str,
        _data: &[u8],
    ) {
    }
}

impl BeaconChip {
    /// Creates a new `BeaconChip`.
    pub fn new(
        rootcanal: &Arc<Bluetooth>,
        params: BeaconCreationParams,
    ) -> Result<(Self, u32), ChipError> {
        let address =
            params.address.parse().unwrap_or_else(|_| Address { address: rand::random() });
        let chip_id = rootcanal.new_controller(address, Box::new(BeaconControllerCallbacks));

        // Reset the controller first.
        let reset_cmd = vec![0x03, 0x0c, 0x00];
        rootcanal.receive_hci(chip_id, Idc::Cmd, &reset_cmd).unwrap();

        // LE Set Advertising Parameters
        let adv_params = vec![
            0x06, 0x20, 15, 0xA0, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x07, 0x00,
        ];
        rootcanal.receive_hci(chip_id, Idc::Cmd, &adv_params).unwrap();

        // LE Set Advertising Data
        let mut adv_data_cmd = vec![0x08, 0x20, 32];
        if !params.adv_data.manufacturer_data.is_empty() {
            adv_data_cmd.push(params.adv_data.manufacturer_data.len() as u8);
            adv_data_cmd.extend_from_slice(&params.adv_data.manufacturer_data);
        } else {
            adv_data_cmd.push(3); // Advertising_Data_Length
            adv_data_cmd.extend_from_slice(&[0x02, 0x01, 0x06]); // Advertising_Data
        }
        adv_data_cmd.resize(3 + 32, 0);
        rootcanal.receive_hci(chip_id, Idc::Cmd, &adv_data_cmd).unwrap();

        // LE Set Advertising Enable
        let adv_enable = vec![0x0A, 0x20, 0x01, 0x01];
        rootcanal.receive_hci(chip_id, Idc::Cmd, &adv_enable).unwrap();

        Ok((Self { chip_id }, chip_id))
    }
}

impl EmulatedChip for BeaconChip {
    fn patch_chip(&mut self, _patch: ChipPatch) -> Result<ChipState, ChipError> {
        self.get_chip()
    }

    fn get_chip(&self) -> Result<ChipState, ChipError> {
        Ok(ChipState { id: self.chip_id })
    }
}
