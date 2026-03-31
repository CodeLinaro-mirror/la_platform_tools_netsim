// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use netsim_packets::{ieee80211::Ieee80211, netlink::hwsim_frame::HwsimFrame};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InfraTarget {
    None,
    Ap,
    Slirp,
}

pub struct TxPacketState {
    pub infra_target: InfraTarget,
    /// delivery to other stations (peers) in the simulated medium.
    /// This covers: unicast to another station, multicast, and broadcast.
    pub stations: bool,
    pub frame: HwsimFrame,
    pub plaintext_ieee80211: Option<Ieee80211>,
    pub plaintext_bytes: Option<Bytes>,
}

impl TxPacketState {
    pub fn get_ieee80211(&self) -> &Ieee80211 {
        self.plaintext_ieee80211.as_ref().unwrap_or(&self.frame.ieee80211)
    }

    pub fn get_ieee80211_bytes(&self) -> Bytes {
        if let Some(bytes) = &self.plaintext_bytes {
            bytes.clone()
        } else if let Some(ieee80211) = &self.plaintext_ieee80211 {
            bytes::Bytes::copy_from_slice(ieee80211.as_bytes())
        } else {
            self.frame.data.clone().into()
        }
    }
}
