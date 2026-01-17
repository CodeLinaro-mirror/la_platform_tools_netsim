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
use netsim_model::chip::Chip;
use netsim_model::chip::{BeaconParams, ChipId};
use netsim_model::chip_error::ChipError;
use rootcanal::Rootcanal;

/// Creates a new `BeaconChip`.
pub fn create(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    params: &BeaconParams,
) -> Result<Chip, ChipError> {
    // Reset the controller first.
    let reset_cmd = vec![0x01, 0x03, 0x0c, 0x00];
    rootcanal.receive_hci(chip_id.into(), reset_cmd.into()).to_chip_error()?;

    // LE Set Advertising Parameters
    let adv_params = vec![
        0x01, 0x06, 0x20, 15, 0xA0, 0x00, 0xA0, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x07, 0x00,
    ];
    rootcanal.receive_hci(chip_id.into(), adv_params.into()).to_chip_error()?;

    // LE Set Advertising Data
    let mut adv_data_cmd = vec![0x01, 0x08, 0x20, 32];
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
    rootcanal.receive_hci(chip_id.into(), adv_data_cmd.into()).to_chip_error()?;

    // LE Set Advertising Enable
    let adv_enable = vec![0x01, 0x0A, 0x20, 0x01, 0x01];
    rootcanal.receive_hci(chip_id.into(), adv_enable.into()).to_chip_error()?;

    Ok(Chip::default())
}
