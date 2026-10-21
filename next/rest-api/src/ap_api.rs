// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Wi-Fi Access Point (AP) REST API endpoints and data transfer models.

pub use netsim_model::{Ap, ApCreate, ApUpdate};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response body for listing Wi-Fi Access Points.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ListApResponse {
    pub aps: Vec<Ap>,
}
