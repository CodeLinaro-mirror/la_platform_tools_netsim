// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::ChipError;

#[derive(thiserror::Error, Debug)]
pub enum NfcError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Missing packet stream or sink")]
    MissingStreamSink,
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),
    #[error("Unknown error")]
    Unknown,
}
