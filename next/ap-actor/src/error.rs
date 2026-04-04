// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::ChipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApError {
    #[error("AP not found: {0}")]
    ApNotFound(u32),
    #[error("AP with ID {0} already exists")]
    ApAlreadyExists(u32),
    #[error("Invalid Frame")]
    InvalidFrame,
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),
    #[error("Internal error: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),
}
