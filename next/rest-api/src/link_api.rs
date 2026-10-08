// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! RF Link REST API endpoints and data transfer models.

pub use netsim_model::{ChipId, Link, LinkCreate, LinkId, LinkUpdate};

pub use crate::ListLinkResponse;

impl From<Vec<Link>> for ListLinkResponse {
    fn from(links: Vec<Link>) -> Self {
        Self { links }
    }
}

/// Request body for creating an RF link.
pub type CreateLinkRequest = LinkCreate;
