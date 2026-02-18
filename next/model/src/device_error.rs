// Copyright (C) 2025 The Android Open Source Project

//! This module defines the error types for the device service.

use thiserror::Error;

use crate::chip_error::ChipError;

/// The error type for operations within the device service.
#[derive(Error, Debug)]
pub enum DeviceError {
    /// An error occurred while communicating with the chip service.
    #[error("Chip service communication error: {0}")]
    ChipClient(String),

    /// An error occurred within the chip service itself.
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),

    /// Error indicating that the arguments provided for an operation were
    /// invalid.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// An internal error occurred within the device service. This typically
    /// indicates a bug or an inconsistent state.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<crate::client_error::ClientError> for DeviceError {
    fn from(err: crate::client_error::ClientError) -> Self {
        match err {
            crate::client_error::ClientError::Send(s) => DeviceError::ChipClient(s),
            crate::client_error::ClientError::Recv(s) => DeviceError::ChipClient(s),
            crate::client_error::ClientError::Chip(e) => DeviceError::Chip(e),
            // A ChipClient should not produce a Device error.
            crate::client_error::ClientError::Device(e) => DeviceError::Internal(format!(
                "Invariant violated: ChipClient returned a DeviceError: {e:?}"
            )),
        }
    }
}
