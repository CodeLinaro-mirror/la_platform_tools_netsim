// Copyright (C) 2025 The Android Open Source Project

//! This module defines the error types for the bluetooth crate.

use thiserror::Error;

/// A specialized `Result` type for bluetooth chip operations.
pub type ChipError = BluetoothError;

/// The error type for the bluetooth crate.
#[derive(Error, Debug)]
pub enum BluetoothError {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The advertising or scan response data exceeds the 31-byte limit.
    #[error("Data exceeds 31-byte limit")]
    DataTooLong,

    /// An invalid input was provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Error indicating that a chip with the given ID already exists.
    #[error("Chip with ID {0} already exists")]
    ChipExists(u32),

    /// Error indicating that a chip with the given ID could not be found.
    #[error("Chip with ID {0} not found")]
    ChipNotFound(u32),

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

    /// Error indicating a failure during protobuf serialization or
    /// deserialization.
    #[error("Protobuf error: {0}")]
    ProtobufError(#[from] protobuf::Error),
}

impl PartialEq for BluetoothError {
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
            (Self::ProtobufError(_), Self::ProtobufError(_)) => true,
            _ => false,
        }
    }
}
