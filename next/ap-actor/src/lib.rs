// Copyright 2025-2026 The Android Open Source Project

pub mod ap_actor;
pub mod ap_client;

pub mod error;
pub mod ffi;
pub mod ieee802_11;
pub mod lifecycle;

pub mod service;
pub mod shared;

pub mod eap_auth;
pub mod ftm;
pub mod rsn;
pub mod sae;
pub mod wpa_auth;

use actor_framework::ResourceActor;
pub use ap_actor::{ApActor, ApConfig, ApReq, ApResponse, ApState, ApUpdate};
pub use ap_client::ApClient;
pub use error::ApError;
pub use netsim_model::{self, device::Position};

/// Creates a new ApActor and returns the runner and a client.
///
/// The runner must be spawned on a runtime.
pub fn new() -> (ResourceActor<ApActor>, ApClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, ApClient::new(client))
}
