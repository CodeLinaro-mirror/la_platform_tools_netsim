// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

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

use netsim_model::{
    chip::{ChipId, DeviceParams},
    chip_error::ChipError,
};
use rootcanal::Rootcanal;

/// Creates a new `DeviceChip` and registers it with the `rootcanal` emulator.
pub fn create(
    _rootcanal: &Rootcanal,
    _chip_id: ChipId,
    _params: &DeviceParams,
) -> Result<(), ChipError> {
    // Validate mode parameters
    Ok(())
}
