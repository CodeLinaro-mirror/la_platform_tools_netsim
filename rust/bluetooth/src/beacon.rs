// Copyright 2023-2025 The Android Open Source Project

use crate::types::EmulatedChip;
use crate::utils::ToChipError;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{BeaconParams, ChipIdentifier};
use netsim_proto::model::Chip as ProtoChip;
use rootcanal::{
    bluetooth::Bluetooth,
    controller::{Callbacks as ControllerCallbacks, Id, Idc},
    types::{Address, Phy},
};
use std::ffi::c_int;
use std::sync::Arc;
use tokio::sync::oneshot;

/// A single BLE beacon chip.
#[allow(dead_code)]
pub(crate) struct BeaconChip {
    chip_id: ChipIdentifier,
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
        chip_id: ChipIdentifier,
        params: &BeaconParams,
    ) -> Result<(Self, Option<oneshot::Sender<()>>), ChipError> {
        let address =
            params.address.parse().unwrap_or_else(|_| Address { address: rand::random() });
        rootcanal
            .new_controller(chip_id.as_u32(), address, Box::new(BeaconControllerCallbacks))
            .to_chip_error()?;

        // Reset the controller first.
        let reset_cmd = vec![0x03, 0x0c, 0x00];
        rootcanal.receive_hci(chip_id.as_u32(), Idc::Cmd, &reset_cmd).unwrap();

        // LE Set Advertising Parameters
        let adv_params = vec![
            0x06, 0x20, 15, 0xA0, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x07, 0x00,
        ];
        rootcanal.receive_hci(chip_id.as_u32(), Idc::Cmd, &adv_params).unwrap();

        // LE Set Advertising Data
        let mut adv_data_cmd = vec![0x08, 0x20, 32];
        if let Some(adv_data) = params.ble_beacon.adv_data.as_ref() {
            if !adv_data.manufacturer_data.is_empty() {
                adv_data_cmd.push(adv_data.manufacturer_data.len() as u8);
                adv_data_cmd.extend_from_slice(&adv_data.manufacturer_data);
            } else {
                adv_data_cmd.push(3); // Advertising_Data_Length
                adv_data_cmd.extend_from_slice(&[0x02, 0x01, 0x06]); // Advertising_Data
            }
        } else {
            adv_data_cmd.push(3); // Advertising_Data_Length
            adv_data_cmd.extend_from_slice(&[0x02, 0x01, 0x06]); // Advertising_Data
        }
        adv_data_cmd.resize(3 + 32, 0);
        rootcanal.receive_hci(chip_id.as_u32(), Idc::Cmd, &adv_data_cmd).unwrap();

        // LE Set Advertising Enable
        let adv_enable = vec![0x0A, 0x20, 0x01, 0x01];
        rootcanal.receive_hci(chip_id.as_u32(), Idc::Cmd, &adv_enable).unwrap();

        Ok((Self { chip_id }, None))
    }
}

impl EmulatedChip for BeaconChip {
    fn update_chip(&mut self, _update: ProtoChip) -> Result<ProtoChip, ChipError> {
        self.get_chip()
    }

    fn get_chip(&self) -> Result<ProtoChip, ChipError> {
        Ok(ProtoChip::default())
    }
}
