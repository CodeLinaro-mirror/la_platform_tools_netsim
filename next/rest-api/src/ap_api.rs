// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Wi-Fi Access Point (AP) REST API endpoints and data transfer models.

pub use netsim_model::{Ap, ApCreate, ApUpdate};
use serde::{Deserialize, Serialize};

/// Response body for listing Wi-Fi Access Points.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ListApResponse {
    pub aps: Vec<Ap>,
}

impl From<Vec<(u32, ap_actor::ApState)>> for ListApResponse {
    fn from(aps: Vec<(u32, ap_actor::ApState)>) -> Self {
        let mut aps: Vec<Ap> = aps
            .into_iter()
            .map(|(id, state)| Ap {
                id,
                config: state.config.into(),
                associations: {
                    // `ApState::associations` is a `HashSet<MacAddr>`, so format each
                    // `MacAddr` and sort to keep the JSON array order stable across calls.
                    let mut associations: Vec<String> =
                        state.associations.into_iter().map(|m| m.to_string()).collect();
                    associations.sort();
                    associations
                },
            })
            .collect();
        aps.sort_by_key(|ap| ap.id);
        Self { aps }
    }
}
