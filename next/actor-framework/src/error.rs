//! # Framework Errors
//!
//! This module defines the common error types used throughout the actor framework.
//! By centralizing error definitions, we ensure consistent error handling across
//! all actors and clients.

/// Errors that can occur within the actor framework itself.
#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    #[error("Actor closed")]
    ActorClosed,
    #[error("Actor dropped response channel")]
    ActorDropped,
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Entity error: {0}")]
    EntityError(Box<dyn std::error::Error + Send + Sync>),
}
