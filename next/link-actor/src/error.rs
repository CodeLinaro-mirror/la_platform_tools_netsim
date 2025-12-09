// Copyright 2025 The Android Open Source Project

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("Link not found")]
    NotFound,
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
