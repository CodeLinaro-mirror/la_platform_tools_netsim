// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! RF Link REST API endpoints and data transfer models.

pub use netsim_model::{ChipId, Link, LinkId, LinkUpdate};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response body for listing RF links.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ListLinkResponse {
    pub links: Vec<Link>,
}

/// Request body for creating an RF link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct CreateLinkRequest {
    pub sender: ChipId,
    pub receiver: ChipId,
    pub rssi: i8,
}
