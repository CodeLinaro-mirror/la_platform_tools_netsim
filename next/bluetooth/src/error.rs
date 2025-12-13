// Copyright 2025 The Android Open Source Project

use netsim_model::chip_error::ChipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BluetoothError {
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),

    #[error("Device client error: {0}")]
    DeviceClient(String),

    #[error("Rootcanal error: {0}")]
    Rootcanal(String),
}
