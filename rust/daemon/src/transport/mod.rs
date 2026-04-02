// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "cuttlefish")]
pub mod fd;
pub mod grpc;
mod h4;
pub mod socket;
// TODO: UWB GF support, then remove the allow
#[allow(dead_code)]
mod uci;
pub mod websocket;

// This provides no-op implementations of fd transport for non-unix systems.
#[cfg(not(feature = "cuttlefish"))]
pub mod fd {
    #[allow(clippy::ptr_arg)]
    pub fn run_fd_transport(_startup_json: &String) {}
}
