// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod ap_actor;
mod ap_client;

mod error;
#[doc(hidden)]
pub mod ffi;
mod ieee802_11;
mod lifecycle;

mod service;
mod shared;

mod eap_auth;
mod ftm;
mod rsn;
mod sae;
mod wpa_auth;

use actor_framework::ResourceActor;

pub use crate::{
    ap_actor::{ApActor, ApConfig, ApReq, ApResponse, ApState, ApUpdate},
    ap_client::ApClient,
    eap_auth::{EapAuthenticator, EapOutput},
    error::ApError,
    sae::{SaeState, SaeStateMachine},
    shared::{SessionKeys, SharedKeyStore},
    wpa_auth::{WpaAuthenticator, WpaOutput},
};

/// Creates a new ApActor and returns the runner and a client.
///
/// The runner must be spawned on a runtime.
pub fn new() -> (ResourceActor<ApActor>, ApClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, ApClient::new(client))
}
