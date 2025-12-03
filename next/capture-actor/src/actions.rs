//! # Capture Actions
//!
//! This module re-exports capture-related types from `netsim-model` for use within the actor.
//! These types define the messages that can be sent to and received from the `CaptureActor`.

pub use capture_api::{CaptureAction, CaptureActionResult, CaptureCreate, CaptureInfo};
