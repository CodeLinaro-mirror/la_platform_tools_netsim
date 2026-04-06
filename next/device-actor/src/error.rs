// Copyright (C) 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Device Errors
//!
//! This module defines the error types specific to the Device actor and client.
//! These errors cover issues arising from actor communication, invalid
//! arguments, and other device-specific failure modes.

use netsim_model::ClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device not found: {0}")]
    NotFound(String),
    #[error("Device not found (explicit): {0}")]
    DeviceNotFound(String),
    #[error("Actor communication error: {0}")]
    ActorCommunicationError(#[from] ClientError),
    #[error("Chip kind not supported: {0}")]
    ChipKindNotSupported(String),
    #[error("Multiple errors during reset: {chip_client_errors:?} {link_client_error:?}")]
    ResetErrors {
        chip_client_errors: Vec<(u32, ClientError)>,
        link_client_error: Option<ClientError>,
    },
}
