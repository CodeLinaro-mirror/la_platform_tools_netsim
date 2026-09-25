// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Netsim HTTP REST API module.
//!
//! Provides REST API request and response models and the HTTP endpoint
//! dispatcher for managing devices, links, and access points.

pub mod ap_api;
pub mod device_api;
pub mod link_api;
pub mod server;

pub use ap_api::*;
pub use device_api::*;
pub use link_api::*;
pub use server::*;
