// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

/// LibSlirp Interface for Network Simulation
use crate::wifi::error::WifiResult;
use bytes::Bytes;
use netsim_proto::config::SlirpOptions as ProtoSlirpOptions;
use std::sync::mpsc;

// Provides a stub implementation while the libslirp-rs crate is not integrated into the aosp-main.
pub struct LibSlirp {}
impl LibSlirp {
    pub fn input(&self, _bytes: Bytes) {}
}

pub fn slirp_run(_opt: ProtoSlirpOptions, _tx_bytes: mpsc::Sender<Bytes>) -> WifiResult<LibSlirp> {
    Ok(LibSlirp {})
}
