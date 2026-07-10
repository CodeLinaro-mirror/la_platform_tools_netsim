// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod error;
mod lifecycle;
mod nfc_actor;
mod scene;
mod service;
mod stats;

mod client;

// Re-export core types
pub use ::casimir;
pub use actor_framework::ResourceActor;
pub use client::NfcClient;
pub use error::NfcError;
pub use nfc_actor::{NfcAction, NfcActor};
pub use scene::SceneClient;
pub use stats::{NfcApi, NfcServiceStats, NfcStats};

/// Creates a new NFC actor framework instance and client.
pub fn new() -> (ResourceActor<NfcActor>, NfcClient) {
    let (runner, client) = ResourceActor::new(32);
    let stats = std::sync::Arc::new(stats::NfcStats::new());
    let service_stats = std::sync::Arc::new(stats::NfcServiceStats::new());
    (runner, NfcClient { client, stats, service_stats })
}
