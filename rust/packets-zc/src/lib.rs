//! `packets_zc` is a crate for zero-copy parsing and handling of network packets.
//!
//! It provides structures and utilities for working with various network protocols,
//! including Ethernet, IEEE 802.11, and Netlink messages specific to `mac80211_hwsim`.
//! The crate emphasizes performance by leveraging the `zerocopy` library.
#![allow(clippy::all)]
#![allow(missing_docs)]
pub mod ethernet;
pub mod ethernet_json;
pub mod ethernet_util;
pub mod ieee80211;
pub mod ieee80211_json;
pub mod ieee80211_util;
pub mod llc;
pub mod mac80211_hwsim_netlink;
pub mod mac80211_hwsim_netlink_json;
pub mod mac80211_hwsim_netlink_util;
#[cfg(test)]
mod tests {}
