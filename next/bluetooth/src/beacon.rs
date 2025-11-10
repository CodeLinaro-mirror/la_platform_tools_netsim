// Copyright 2023-2025 The Android Open Source Project

//! This module provides the functionality for creating and managing Bluetooth
//! beacon chips.
//!
//! It includes the `create` function for initializing a new beacon with
//! advertising parameters and data, and placeholder functions for updating
//! and retrieving chip information.
//!
//! NOTE: This module is currently missing the complete setup for converting
//! `BeaconParams` into the appropriate HCI commands for full configuration.

use crate::utils::ToChipError;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::Chip;
use netsim_api::chips::{BeaconParams, ChipId};
use rootcanal::{controller::Idc, Rootcanal};

/// Creates a new `BeaconChip`.
pub fn create(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    params: &BeaconParams,
) -> Result<(), ChipError> {
    // Reset the controller first.
    let reset_cmd = vec![0x03, 0x0c, 0x00];
    rootcanal.receive_hci(chip_id.into(), Idc::Cmd, &reset_cmd).to_chip_error()?;

    // LE Set Advertising Parameters
    let adv_params = vec![
        0x06, 0x20, 15, 0xA0, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x07, 0x00,
    ];
    rootcanal.receive_hci(chip_id.into(), Idc::Cmd, &adv_params).to_chip_error()?;

    // LE Set Advertising Data
    let mut adv_data_cmd = vec![0x08, 0x20, 32];
    if let Some(adv_data) = &params.ble_beacon.adv_data {
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
    rootcanal.receive_hci(chip_id.into(), Idc::Cmd, &adv_data_cmd).to_chip_error()?;

    // LE Set Advertising Enable
    let adv_enable = vec![0x0A, 0x20, 0x01, 0x01];
    rootcanal.receive_hci(chip_id.into(), Idc::Cmd, &adv_enable).to_chip_error()?;

    Ok(())
}

#[allow(dead_code)]
/// NOTE: This function is a stub and not fully implemented.
pub fn update_chip(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    _old_params: &BeaconParams,
    _new_params: &BeaconParams,
) -> Result<Chip, ChipError> {
    get_chip(rootcanal, chip_id)
}

#[allow(dead_code)]
/// NOTE: This function is a stub and not fully implemented.
pub fn get_chip(_rootcanal: &Rootcanal, _chip_id: ChipId) -> Result<Chip, ChipError> {
    Ok(Chip::default())
}
