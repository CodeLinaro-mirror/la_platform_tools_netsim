// Copyright 2024-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs::ModemError;
use netsim_model::chip_error::ChipError;

#[derive(thiserror::Error, Debug)]
pub enum CellError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Missing packet stream or sink")]
    MissingStreamSink,
    #[error("Controller service error: {0}")]
    ModemError(#[from] ModemError),
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),
    #[error("Unknown error")]
    Unknown,
}
