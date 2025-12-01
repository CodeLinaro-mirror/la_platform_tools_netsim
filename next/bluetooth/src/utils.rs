// Copyright 2023-2025 The Android Open Source Project

//! This module provides utility functions and traits for the bluetooth crate.
//!
//! It includes helpers for error conversion, such as the `ToChipError` trait,
//! which simplifies the mapping of `rootcanal` emulator errors to the `ChipError` type.

use netsim_model::chip_error::ChipError;
use rootcanal::error::Error as RadioError;

/// A helper trait to simplify error conversion from `RadioError` to `ChipError`.
pub(crate) trait ToChipError<T> {
    fn to_chip_error(self) -> Result<T, ChipError>;
}

impl<T> ToChipError<T> for Result<T, RadioError> {
    fn to_chip_error(self) -> Result<T, ChipError> {
        self.map_err(|e| ChipError::BackendError(e.to_string()))
    }
}
