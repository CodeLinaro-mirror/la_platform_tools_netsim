// Copyright 2025 The Android Open Source Project

//! This module defines the [`enum@Error`] enum, which represents all possible
//! errors that can occur in this crate.

use std::{io, num::ParseIntError};

use thiserror::Error;

/// Represents all possible errors that can occur in this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An I/O error.
    #[error(transparent)]
    IoError(#[from] io::Error),

    /// The controller ID is already in use.
    #[error("duplicate controller id: {0}")]
    DuplicateControllerId(u32),

    /// The controller ID is not found.
    #[error("controller not found: {0}")]
    ControllerNotFound(u32),

    /// The address is malformed.
    #[error("malformed address: {0}")]
    MalformedAddress(String),

    /// The class of device is malformed.
    #[error("malformed class of device: {0}")]
    MalformedClassOfDevice(String),

    /// The LE features are malformed.
    #[error("malformed LE features: {0}")]
    MalformedLeFeatures(String),

    /// An error parsing an integer.
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}

/// A type alias for `Result` where the error type is this crate's `Error`.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_chaining() {
        let inner_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let outer_error: Error = inner_error.into();

        assert!(outer_error.to_string().contains("file not found"));
    }
}
