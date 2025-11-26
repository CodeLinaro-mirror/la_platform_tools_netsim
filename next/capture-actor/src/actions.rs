//! # Capture Actions
//!
//! This module re-exports capture-related types from `netsim-model` for use within the actor.
//! These types define the messages that can be sent to and received from the `CaptureActor`.

pub use netsim_model::capture::{CaptureAction, CaptureActionResult, CaptureCreate, CaptureInfo};
