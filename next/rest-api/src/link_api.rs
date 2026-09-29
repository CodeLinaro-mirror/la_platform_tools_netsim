// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! RF Link REST API endpoints and data transfer models.

pub use netsim_model::{ChipId, Link, LinkCreate, LinkId, LinkUpdate};
use serde::{Deserialize, Serialize};

/// Response body for listing RF links.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListLinkResponse {
    pub links: Vec<Link>,
}

impl From<Vec<Link>> for ListLinkResponse {
    fn from(links: Vec<Link>) -> Self {
        Self { links }
    }
}

/// Request body for creating an RF link.
pub type CreateLinkRequest = LinkCreate;
