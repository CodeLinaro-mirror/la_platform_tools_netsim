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
    hostapd_error: AtomicU32,
    network_error: AtomicU32,
    client_error: AtomicU32,
    frame_error: AtomicU32,
    transmission_error: AtomicU32,
    other_error: AtomicU32,
}

impl WifiStats {
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
}

fn load_as_option_i32(counter: &AtomicU32) -> Option<i32> {
    Some(counter.load(Ordering::Relaxed) as i32)
}

impl From<&WifiStats> for ProtoWifiStats {
    fn from(wifi_stats: &WifiStats) -> Self {
        let counts = &wifi_stats.counts;
        ProtoWifiStats {
            hostapd_errors: load_as_option_i32(&counts.hostapd_error),
            network_errors: load_as_option_i32(&counts.network_error),
            client_errors: load_as_option_i32(&counts.client_error),
            frame_errors: load_as_option_i32(&counts.frame_error),
            transmission_errors: load_as_option_i32(&counts.transmission_error),
            other_errors: load_as_option_i32(&counts.other_error),
            ..Default::default()
        }
    }
}
