// Copyright 2023-2025 The Android Open Source Project

//! This module provides the functionality for creating and managing Bluetooth
//! device chips.
//!
//! A 'Device' chip represents a Bluetooth controller that can be controlled by
//! an external entity, such as an Android Virtual Device running in an Android
//! Emulator or a host device like those supported by Bumble.
//!
//! The `create` function is responsible for initializing a new device chip.
//! Placeholder functions for updating and retrieving chip information are also
//! included.

use netsim_api::chip_error::ChipError;
use netsim_api::chips::{ChipId, DeviceParams};
use netsim_proto::model::Chip as ProtoChip;
use rootcanal::bluetooth::Bluetooth as Rootcanal;

/// Creates a new `DeviceChip` and registers it with `rootcanal`.
pub fn create(
    _rootcanal: &Rootcanal,
    _chip_id: ChipId,
    _params: &DeviceParams,
) -> Result<(), ChipError> {
    // Validate mode parameters
    Ok(())
}

#[allow(dead_code)]
pub fn update_chip(
    rootcanal: &Rootcanal,
    chip_id: ChipId,
    _old_params: &DeviceParams,
    _new_params: &DeviceParams,
) -> Result<ProtoChip, ChipError> {
    get_chip(rootcanal, chip_id)
}

#[allow(dead_code)]
pub fn get_chip(_rootcanal: &Rootcanal, _chip_id: ChipId) -> Result<ProtoChip, ChipError> {
    Ok(ProtoChip::default())
}
