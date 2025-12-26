// Copyright 2023-2025 The Android Open Source Project

//! This module provides the functionality for creating and managing Bluetooth
//! sniffer chips.
//!
//! The `create` function initializes a new sniffer by enabling scanning.
//! Placeholder functions for updating and retrieving chip information are also
//! included.
//!
//! Future Features:
//! * **External Link Layer API:** Currently used for testing, this mode may expose an external API
//!   in the future to convert Rootcanal LL packets to standard Bluetooth LL packets for capture.

use crate::utils::ToChipError;
use log::debug;
use netsim_api::chip_error::ChipError;
use netsim_api::chips::{Chip, ChipId, SnifferParams};
use rootcanal::{controller::Idc, Rootcanal};

/// A stateless struct that provides the behavior for a Bluetooth sniffer.
pub(crate) fn create(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    _params: &SnifferParams,
) -> Result<(), ChipError> {
    debug!("[{chip_id}] Setting up sniffer chip");
    // Enable scanning on the new controller.
    debug!("[{chip_id}] Enabling scanning");
    let scan_params = vec![0x12, 0x20, 7, 0x01, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00];
    rootcanal.receive_hci(chip_id.into(), Idc::Cmd, &scan_params).to_chip_error()?;
    let scan_enable = vec![0x0c, 0x20, 2, 0x01, 0x00];
    rootcanal.receive_hci(chip_id.into(), Idc::Cmd, &scan_enable).to_chip_error()?;
    Ok(())
}

#[allow(dead_code)]
pub fn update_chip(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    _old_params: &SnifferParams,
    _new_params: &SnifferParams,
) -> Result<Chip, ChipError> {
    get_chip(rootcanal, chip_id)
}

#[allow(dead_code)]
pub fn get_chip(_rootcanal: &Rootcanal, _chip_id: ChipId) -> Result<Chip, ChipError> {
    Ok(Chip::default())
}
