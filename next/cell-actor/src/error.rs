// Copyright 2024-2025 The Android Open Source Project

use netsim_model::chip_error::ChipError;

#[derive(thiserror::Error, Debug)]
pub enum CellError {
    #[error("Failed to send message: {0}")]
    SendError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Missing packet stream or sink")]
    MissingStreamSink,
    #[error("Controller service error: {0}")]
    ModemError(Box<dyn std::error::Error + Send + Sync>),
    #[error("Chip error: {0}")]
    Chip(#[from] ChipError),
    #[error("Unknown error")]
    Unknown,
}
