// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod ap_actor;
mod ap_client;
mod eap_auth;
mod error;
#[doc(hidden)]
pub mod ffi;
mod ftm;
mod ieee802_11;
mod lifecycle;
mod rsn;
mod sae;
mod service;
mod shared;
mod wpa_auth;

use actor_framework::ResourceActor;

pub use crate::{
    ap_actor::{
        ApActor, ApActorUpdate, ApConfig, ApReq, ApResponse, ApState, ApUpdate, DEFAULT_AP_BSSID,
        EXPECTED_DEFAULT_AP_BSSID, expected_default_ap_bssid,
    },
    ap_client::ApClient,
    error::ApError,
    sae::SaeStateMachine,
    shared::SharedKeyStore,
};

/// Creates a new ApActor and returns the runner and a client.
///
/// The runner must be spawned on a runtime.
pub fn new() -> (ResourceActor<ApActor>, ApClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, ApClient::new(client))
}
