// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Netsim HTTP REST API module.
//!
//! Provides REST API request and response models and the HTTP endpoint
//! dispatcher for managing devices, links, and access points.

use serde::{Deserialize, Serialize};

pub mod ap_api;
pub mod client;
pub mod device_api;
pub mod link_api;
pub mod server;

pub use ap_api::*;
pub use client::*;
pub use device_api::*;
pub use link_api::*;
pub use server::*;

/// Endpoint paths shared by the REST server and client, so the two cannot
/// drift apart. Usable as `match` patterns.
pub mod path {
    pub const VERSION: &str = "/v1/version";
    pub const DEVICES: &str = "/v1/devices";
    pub const LINKS: &str = "/v1/links";
    pub const APS: &str = "/v1/aps";
}

/// Response body for `GET /v1/version`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
}

/// Standard JSON error response body returned by REST endpoints on failure.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
