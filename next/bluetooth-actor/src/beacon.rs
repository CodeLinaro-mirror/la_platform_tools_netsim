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

use netsim_model::{
    chip::{BeaconParams, Chip, ChipId},
    chip_error::ChipError,
};
use rootcanal::Rootcanal;

use crate::{beacon_utils::construct_data, utils::ToChipError};

/// Creates a new `BeaconChip`.
pub fn create(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    params: &BeaconParams,
    device_name: &Option<String>,
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
    let adv_data_payload = if let Some(adv_data) = &params.ble_beacon.adv_data {
        construct_data(
            &adv_data.manufacturer_data,
            &if adv_data.include_device_name { device_name.clone() } else { None },
        )
    } else {
        construct_data(&[], &None)
    };

    // Length of significant data
    adv_data_cmd.push(adv_data_payload.len() as u8);
    adv_data_cmd.extend_from_slice(&adv_data_payload);

    // HCI packet must be exactly 36 bytes (4 header + 32 data)
    adv_data_cmd.resize(36, 0);
    rootcanal.receive_hci(chip_id.into(), adv_data_cmd.into()).to_chip_error()?;

    // LE Set Scan Response Data
    let mut scan_resp_cmd = vec![0x01, 0x09, 0x20, 32];
    let scan_resp_payload = if let Some(scan_resp) = &params.ble_beacon.scan_response {
        construct_data(
            &scan_resp.manufacturer_data,
            &if scan_resp.include_device_name { device_name.clone() } else { None },
        )
    } else {
        Vec::new()
    };

    scan_resp_cmd.push(scan_resp_payload.len() as u8);
    scan_resp_cmd.extend_from_slice(&scan_resp_payload);

    // HCI packet must be exactly 36 bytes (4 header + 32 data)
    scan_resp_cmd.resize(36, 0);
    rootcanal.receive_hci(chip_id.into(), scan_resp_cmd.into()).to_chip_error()?;

    // LE Set Advertising Enable
    let adv_enable = vec![0x01, 0x0A, 0x20, 0x01, 0x01];
    rootcanal.receive_hci(chip_id.into(), adv_enable.into()).to_chip_error()?;

    Ok(Chip::default())
}
