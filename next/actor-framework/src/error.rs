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
    ActorClosed(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("Actor dropped response channel")]
    ActorDropped(#[source] tokio::sync::oneshot::error::RecvError),
    #[error("Service error: {0}")]
    ServiceError(#[source] E),
}

impl<E: 'static + Error + Send + Sync> From<FrameworkError<E>> for ClientError {
    fn from(val: FrameworkError<E>) -> Self {
        ClientError::Framework(Box::new(val))
    }
}
