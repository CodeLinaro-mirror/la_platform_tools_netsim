// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Wi-Fi Statistics Module
//! This module provides structures and functions for tracking and managing Wi-Fi related statistics.

use crate::wifi::error::WifiError;
use log::warn;
use netsim_proto::stats::WifiStats as ProtoWifiStats;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct WifiStats {
    counts: Arc<WifiCounts>,
}

impl Clone for WifiStats {
    fn clone(&self) -> Self {
        WifiStats { counts: self.counts.clone() }
    }
}

#[derive(Debug, Default)]
struct WifiCounts {
    // === Error Counters (Proto fields 1-6) ===
    hostapd_error: AtomicU32,
    network_error: AtomicU32,
    client_error: AtomicU32,
    frame_error: AtomicU32,
    transmission_error: AtomicU32,
    other_error: AtomicU32,

    // === Core Traffic Flow & Type Counters (Proto fields 7-14) ===
    hwsim_frames_rx: AtomicU32,
    hwsim_frames_tx: AtomicU32,
    network_packets_tx: AtomicU32,
    network_packets_rx: AtomicU32,
    hostapd_frames_tx: AtomicU32,
    hostapd_frames_rx: AtomicU32,
    wmedium_frames_tx: AtomicU32,
    wmedium_unicast_frames_tx: AtomicU32,
    mgmt_frames_rx: AtomicU32,

    // === Specific Protocol Counters (Proto fields 15-16) ===
    // TODO: Identify Wi-Fi Direct (P2P) frames in medium.rs and increment this counter.
    mdns_count: AtomicU32,
}

// Define the macro to generate incrementer methods
macro_rules! impl_incr_method {
    // $method: Name of the function to generate (e.g., incr_hwsim_frames_rx)
    // $field: Name of the field in WifiCounts to increment (e.g., hwsim_frames_rx)
    ($method:ident, $field:ident) => {
        pub fn $method(&self) {
            self.counts.$field.fetch_add(1, Ordering::Relaxed);
        }
    };
}

impl WifiStats {
    /// Logs the error and increments the corresponding counter.
    pub fn log_and_incr_err_count(&self, error: &WifiError) {
        warn!("{}", error);
        let counter = match error {
            WifiError::Hostapd(_) => &self.counts.hostapd_error,
            WifiError::Network(_) => &self.counts.network_error,
            WifiError::Client(_) => &self.counts.client_error,
            WifiError::Frame(_) => &self.counts.frame_error,
            WifiError::Transmission(_) => &self.counts.transmission_error,
            _ => &self.counts.other_error,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    // Use the macro to generate the incrementer methods for usage counters
    impl_incr_method!(incr_hwsim_frames_rx, hwsim_frames_rx);
    impl_incr_method!(incr_hwsim_frames_tx, hwsim_frames_tx);
    impl_incr_method!(incr_network_packets_tx, network_packets_tx);
    impl_incr_method!(incr_network_packets_rx, network_packets_rx);
    impl_incr_method!(incr_hostapd_frames_tx, hostapd_frames_tx);
    impl_incr_method!(incr_hostapd_frames_rx, hostapd_frames_rx);
    impl_incr_method!(incr_wmedium_frames_tx, wmedium_frames_tx);
    impl_incr_method!(incr_wmedium_unicast_frames_tx, wmedium_unicast_frames_tx);
    impl_incr_method!(incr_mgmt_frames_rx, mgmt_frames_rx);
    impl_incr_method!(incr_mdns_count, mdns_count);
}

fn load_as_option_i32(counter: &AtomicU32) -> Option<i32> {
    Some(counter.load(Ordering::Relaxed) as i32)
}

impl From<&WifiStats> for ProtoWifiStats {
    fn from(wifi_stats: &WifiStats) -> Self {
        let counts = &wifi_stats.counts;
        ProtoWifiStats {
            // Errors
            hostapd_errors: load_as_option_i32(&counts.hostapd_error),
            network_errors: load_as_option_i32(&counts.network_error),
            client_errors: load_as_option_i32(&counts.client_error),
            frame_errors: load_as_option_i32(&counts.frame_error),
            transmission_errors: load_as_option_i32(&counts.transmission_error),
            other_errors: load_as_option_i32(&counts.other_error),

            // Core Flow & Type
            hwsim_frames_rx: load_as_option_i32(&counts.hwsim_frames_rx),
            hwsim_frames_tx: load_as_option_i32(&counts.hwsim_frames_tx),
            network_packets_tx: load_as_option_i32(&counts.network_packets_tx),
            network_packets_rx: load_as_option_i32(&counts.network_packets_rx),
            hostapd_frames_tx: load_as_option_i32(&counts.hostapd_frames_tx),
            hostapd_frames_rx: load_as_option_i32(&counts.hostapd_frames_rx),
            wmedium_frames_tx: load_as_option_i32(&counts.wmedium_frames_tx),
            wmedium_unicast_frames_tx: load_as_option_i32(&counts.wmedium_unicast_frames_tx),
            mgmt_frames_rx: load_as_option_i32(&counts.mgmt_frames_rx),

            // Specific Protocols
            mdns_count: load_as_option_i32(&counts.mdns_count),

            ..Default::default()
        }
    }
}
