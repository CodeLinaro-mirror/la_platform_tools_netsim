// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Netsim daemon libraries.

use std::sync::OnceLock;
use tokio::runtime::{Handle, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Retrieves a handle to a shared, lazily initialized Tokio runtime.
pub fn get_runtime() -> Handle {
    RUNTIME.get_or_init(|| Runtime::new().unwrap()).handle().clone()
}

mod args;
mod bluetooth;
pub mod captures;
mod config_file;
mod devices;
mod events;
mod ffi;
mod grpc_server;
mod http_server;
mod links;
mod proto_mapping;
mod ranging;
mod resource;
mod rust_main;
mod service;
mod session;
mod transport;
mod uwb;
mod version;
mod websocket_server;
mod wifi;
mod wireless;

// This feature is enabled only for CMake builds
#[cfg(feature = "local_ssl")]
mod openssl;
