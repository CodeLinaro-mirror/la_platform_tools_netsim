// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Framework Errors
//!
//! This module defines the common error types used throughout the actor
//! framework. By centralizing error definitions, we ensure consistent error
//! handling across all actors and clients.

use std::error::Error;

use netsim_model::ClientError;

/// Errors that can occur within the actor framework itself.
#[derive(Debug, thiserror::Error)]
pub enum FrameworkError<E: Error + Send + Sync> {
    #[error("Actor send channel closed")]
    ActorClosed,
    #[error("Actor dropped response channel")]
    ActorDropped(#[source] tokio::sync::oneshot::error::RecvError),
    #[error("Service error: {0}")]
    ServiceError(#[source] E),
}

impl<E: 'static + Error + Send + Sync> Into<ClientError> for FrameworkError<E> {
    fn into(self) -> ClientError {
        ClientError::Framework(Box::new(self))
    }
}
