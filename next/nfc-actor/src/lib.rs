// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod error;
mod lifecycle;
mod nfc_actor;
mod scene;
mod service;

mod client;

// Re-export core types
pub use actor_framework::ResourceActor;
pub use client::NfcClient;
pub use error::NfcError;
pub use nfc_actor::NfcActor;

/// Creates a new NFC actor framework instance and client.
pub fn new() -> (ResourceActor<NfcActor>, NfcClient) {
    let (runner, client) = ResourceActor::new(32);
    (runner, NfcClient(client))
}
