// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::chip_error::ChipError;
use thiserror::Error;

/// Error type for CaptureActor operations.
#[derive(Error, Debug)]
pub enum CaptureError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Chip(#[from] ChipError),
}
