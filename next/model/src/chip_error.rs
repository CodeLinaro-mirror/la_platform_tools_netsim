// Copyright (C) 2025 The Android Open Source Project

//! This module defines the error types for the chip service.

use thiserror::Error;

use crate::chip::{ChipId, ChipKind};

/// The error type for operations within the chip service.
#[derive(Error, Debug)]
pub enum ChipError {
    /// Error indicating that a chip with the given ID was not found.
    #[error("Chip not found error: {0}")]
    ChipNotFound(ChipId),

    /// An error occurred during I/O.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The operation is not supported.
    #[error("Unsupported operation")]
    Unsupported,

    /// Error indicating that a chip with the given ID already exists.
    #[error("Chip with ID {0} already exists")]
    ChipExists(u32),

    /// A catch-all for errors originating from the simulation backend.
    #[error("Backend error: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Error indicating an internal error.
    #[error("Internal error: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Wrong Chip Variant")]
    WrongVariantError,

    /// Invalid address.
    #[error("Invalid address: {0}")]
    InvalidAddress(Box<dyn std::error::Error + Send + Sync>),

    /// Unexpected chip kind.
    #[error("Unexpected chip kind: expected {expected:?}, got {actual:?}")]
    UnexpectedChipKind { expected: ChipKind, actual: ChipKind },

    /// Invalid arguments.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(Box<dyn std::error::Error + Send + Sync>),
}
