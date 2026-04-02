// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # netsim-packets Crate
//!
//! A collection of packet definitions for netsimd.

pub mod ieee80211;
pub mod llc;

pub mod link_layer {
    #![allow(clippy::all)]
    #![allow(unused)]
    #![allow(missing_docs)]

    include!(concat!(env!("OUT_DIR"), "/link_layer_packets.rs"));
}

pub mod netlink {
    #![allow(clippy::all)]
    #![allow(unused)]
    #![allow(missing_docs)]

    include!(concat!(env!("OUT_DIR"), "/netlink_packets.rs"));
}

pub mod mac80211_hwsim {
    #![allow(clippy::all)]
    #![allow(unused)]
    #![allow(missing_docs)]

    include!(concat!(env!("OUT_DIR"), "/mac80211_hwsim_packets.rs"));
}
