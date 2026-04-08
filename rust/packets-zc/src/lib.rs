// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! `packets_zc` is a crate for zero-copy parsing and handling of network packets.
//!
//! It provides structures and utilities for working with various network protocols,
//! including Ethernet, IEEE 802.11, and Netlink messages specific to `mac80211_hwsim`.
//! The crate emphasizes performance by leveraging the `zerocopy` library.
#![allow(missing_docs)]
pub mod ethernet;
pub mod ethernet_json;
pub mod ethernet_util;
pub mod icmp;
pub mod icmp_json;
pub mod icmpv6;
pub mod icmpv6_json;
pub mod ieee80211;
pub mod ieee80211_json;
pub mod ieee80211_util;
pub mod ip;
pub mod ip_json;
pub mod json_common;
pub mod llc;
pub mod llc_json;
pub mod llc_util;
pub mod nl80211;
pub mod nl80211_attr;
pub mod nl80211_json;
pub mod nl80211_util;
pub mod packet;
pub mod packet_json;
pub mod pcapng;
pub mod tcp;
pub mod udp;
pub mod util;
