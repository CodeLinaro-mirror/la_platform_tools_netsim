// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::ChipError;
use thiserror::Error;

/// Errors that can occur within the Bluetooth actor.
#[derive(Error, Debug)]
pub enum BluetoothError {
    /// Errors related to general chip operations.
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),

    /// Errors related to the device client.
    #[error("Device client error: {0}")]
    DeviceClient(Box<dyn std::error::Error + Send + Sync>),

    /// Errors originating from the Rootcanal simulation.
    #[error("Rootcanal error: {0}")]
    Rootcanal(Box<dyn std::error::Error + Send + Sync>),

    /// Error parsing Bluetooth address.
    #[error("Address parse error: {0}")]
    AddressParse(#[source] ::rootcanal::error::Error),
}

impl BluetoothError {
    /// Helper to create an InvalidArguments error.
    pub fn invalid_arg(msg: impl Into<String>) -> Self {
        Self::Chip(ChipError::InvalidArguments(Box::from(msg.into())))
    }
}
