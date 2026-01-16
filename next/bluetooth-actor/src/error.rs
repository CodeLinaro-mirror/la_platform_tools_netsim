// Copyright 2025 The Android Open Source Project

use netsim_model::chip_error::ChipError;
use thiserror::Error;

/// Errors that can occur within the Bluetooth actor.
#[derive(Error, Debug)]
pub enum BluetoothError {
    /// Errors related to general chip operations.
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),

    /// Errors related to the device client.
    #[error("Device client error: {0}")]
    DeviceClient(String),

    /// Errors originating from the Rootcanal simulation.
    #[error("Rootcanal error: {0}")]
    Rootcanal(String),
}
