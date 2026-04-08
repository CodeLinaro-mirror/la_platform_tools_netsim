// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

/// Hostapd Interface for Network Simulation
use crate::wifi::error::WifiResult;
use bytes::Bytes;
use netsim_packets::ieee80211::{Ieee80211, MacAddress};
use netsim_proto::config::HostapdOptions as ProtoHostapdOptions;
use tokio::sync::mpsc;

// Provides a stub implementation while the hostapd-rs crate is not integrated into the aosp-main.
pub struct Hostapd {}
impl Hostapd {
    pub async fn input(&self, _bytes: Bytes) -> WifiResult<()> {
        Ok(())
    }

    /// Retrieves the `Hostapd`'s BSSID.
    pub fn get_bssid(&self) -> MacAddress {
        MacAddress::try_from(0).unwrap()
    }

    /// Attempt to encrypt the given IEEE 802.11 frame.
    pub fn try_encrypt(&self, _ieee80211: &Ieee80211) -> Option<Ieee80211> {
        None
    }

    /// Attempt to decrypt the given IEEE 802.11 frame.
    pub fn try_decrypt(&self, _ieee80211: &Ieee80211) -> Option<Ieee80211> {
        None
    }
}

pub async fn hostapd_run(
    _opt: ProtoHostapdOptions,
    _tx: mpsc::Sender<Bytes>,
    _wifi_args: Option<Vec<String>>,
) -> WifiResult<Hostapd> {
    Ok(Hostapd {})
}
