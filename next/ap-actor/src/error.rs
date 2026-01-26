// Copyright 2025-2026 The Android Open Source Project

use thiserror::Error;

#[derive(Error, Debug)]

pub enum ApError {
    #[error("AP not found: {0}")]
    ApNotFound(u32),
    #[error("AP already exists")]
    ApAlreadyExists,
    #[error("Invalid Frame")]
    InvalidFrame,
    #[error("Internal error: {0}")]
    Internal(String),
}
