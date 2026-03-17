// Copyright 2024-2025 The Android Open Source Project
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CellError {
    #[error("Chip not found: {0}")]
    ChipNotFound(String),
    #[error("Failed to send message: {0}")]
    SendError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Missing packet stream or sink")]
    MissingStreamSink,
    #[error("Controller service error: {0}")]
    ModemError(String),
    #[error("Unknown error")]
    Unknown,
}
