// Copyright 2025 The Android Open Source Project

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("Link not found: {0}")]
    NotFound(String),
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Link already exists: {0}")]
    AlreadyExists(String),
    #[error("Actor communication error: {0}")]
    ActorCommunicationError(String),
}

impl From<String> for LinkError {
    fn from(s: String) -> Self {
        LinkError::Internal(s)
    }
}
