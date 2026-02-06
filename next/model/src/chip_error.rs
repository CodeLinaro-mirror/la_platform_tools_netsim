// Copyright (C) 2025 The Android Open Source Project

//! This module defines the error types for the chip service.

use crate::chip::ChipId;

use thiserror::Error;

/// The error type for operations within the chip service.
#[derive(Error, Debug)]
pub enum ChipError {
    /// Error indicating that a chip with the given ID was not found.
    #[error("Chip not found error: {0}")]
    ChipNotFound(ChipId),

    /// An error occurred during I/O.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred during packet processing.
    #[error("Packet processing error: {0}")]
    Packet(String),

    /// The operation is not supported.
    #[error("Unsupported operation")]
    Unsupported,

    /// The advertising or scan response data exceeds the 31-byte limit.
    #[error("Data exceeds 31-byte limit")]
    DataTooLong,

    /// An invalid input was provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Error indicating that a chip with the given ID already exists.
    #[error("Chip with ID {0} already exists")]
    ChipExists(u32),

    /// Error indicating that a patch operation failed due to invalid data.
    #[error("Invalid patch: {0}")]
    InvalidPatch(String),

    /// Error indicating that the arguments provided for an operation were
    /// invalid.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// A catch-all for errors originating from the simulation backend.
    #[error("Backend error: {0}")]
    BackendError(String),

    /// Radio error.
    #[error("Radio error: {0}")]
    RadioError(String),

    /// Error indicating an internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Wrong Chip Variant")]
    WrongVariantError,
}

impl PartialEq for ChipError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(l), Self::Io(r)) => l.kind() == r.kind(),
            (Self::DataTooLong, Self::DataTooLong) => true,
            (Self::InvalidInput(l), Self::InvalidInput(r)) => l == r,
            (Self::ChipExists(l0), Self::ChipExists(r0)) => l0 == r0,
            (Self::ChipNotFound(l0), Self::ChipNotFound(r0)) => l0 == r0,
            (Self::InvalidPatch(l0), Self::InvalidPatch(r0)) => l0 == r0,
            (Self::InvalidArguments(l0), Self::InvalidArguments(r0)) => l0 == r0,
            (Self::BackendError(l0), Self::BackendError(r0)) => l0 == r0,
            _ => false,
        }
    }
}
