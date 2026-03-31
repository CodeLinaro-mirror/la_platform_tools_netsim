// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("Link not found: {0}")]
    NotFound(u32),
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
    #[error("Internal error: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),
    #[error("Link already exists")]
    AlreadyExists,
    #[error("Actor communication error: {0}")]
    ActorCommunicationError(Box<dyn std::error::Error + Send + Sync>),
}

impl From<String> for LinkError {
    fn from(s: String) -> Self {
        LinkError::Internal(Box::from(s))
    }
}
