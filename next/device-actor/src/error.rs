// Copyright (C) 2025 The Android Open Source Project

//! Device Errors
//!
//! This module defines the error types specific to the Device actor and client.
//! These errors cover issues arising from actor communication, invalid arguments,
//! and other device-specific failure modes.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device not found: {0}")]
    NotFound(String),
    #[error("Device not found (explicit): {0}")]
    DeviceNotFound(String),
    #[error("Actor communication error: {0}")]
    ActorCommunicationError(String),
}

impl From<String> for DeviceError {
    fn from(msg: String) -> Self {
        DeviceError::ActorCommunicationError(msg)
    }
}
