// Copyright 2025 Google LLC

use crate::error::WifiError;
use log::{debug, warn};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Stats for the Wifi Actor.
///
/// Tracks error counts, packet counts, and throughput metrics.
/// Designed for single-threaded usage within the actor (no internal locking).
#[derive(Clone, Debug, Default)]
pub struct WifiStats {
    counts: WifiCounts,
    values: WifiValues,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
struct WifiCounts {
    hostapd_error: u64,
    network_error: u64,
    client_error: u64,
    frame_error: u64,
    transmission_error: u64,
    other_error: u64,

    hwsim_frames_rx: u64,
    hwsim_frames_tx: u64,
    network_packets_tx: u64,
    network_packets_rx: u64,
    hostapd_frames_tx: u64,
    hostapd_frames_rx: u64,
    wmedium_frames_tx: u64,
    wmedium_unicast_frames_tx: u64,
    mgmt_frames_rx: u64,
    mdns_count: u64,
}

#[derive(Clone, Debug, Default)]
struct ThroughputValues {
    max_throughput: u64,
    window_start: u64,
    window_bytes: u64,
}

#[derive(Clone, Debug, Default)]
struct WifiValues {
    download_throughput: ThroughputValues,
    upload_throughput: ThroughputValues,
}

impl WifiStats {
    pub fn record_download_bytes(&mut self, bytes: usize) {
        self.values.download_throughput.record(bytes, "Download");
    }

    pub fn record_upload_bytes(&mut self, bytes: usize) {
        self.values.upload_throughput.record(bytes, "Upload");
    }

    pub fn log_and_incr_err_count(&mut self, error: &WifiError) {
        warn!("{error}");
        self.counts.other_error += 1;
    }

    pub fn incr_hwsim_frames_rx(&mut self) {
        self.counts.hwsim_frames_rx += 1;
    }

    pub fn incr_hwsim_frames_tx(&mut self) {
        self.counts.hwsim_frames_tx += 1;
    }

    pub fn incr_network_packets_tx(&mut self) {
        self.counts.network_packets_tx += 1;
    }

    pub fn incr_network_packets_rx(&mut self) {
        self.counts.network_packets_rx += 1;
    }

    pub fn incr_hostapd_frames_tx(&mut self) {
        self.counts.hostapd_frames_tx += 1;
    }

    pub fn incr_hostapd_frames_rx(&mut self) {
        self.counts.hostapd_frames_rx += 1;
    }

    pub fn incr_wmedium_frames_tx(&mut self) {
        self.counts.wmedium_frames_tx += 1;
    }

    pub fn incr_wmedium_unicast_frames_tx(&mut self) {
        self.counts.wmedium_unicast_frames_tx += 1;
    }

    pub fn incr_mgmt_frames_rx(&mut self) {
        self.counts.mgmt_frames_rx += 1;
    }

    pub fn incr_mdns_count(&mut self) {
        self.counts.mdns_count += 1;
    }

    pub fn bytes_ps_to_mbps(bytes_per_second: u64) -> f32 {
        ((bytes_per_second as f64 * 8.0) / (1_000_000.0)) as f32
    }
}

impl ThroughputValues {
    fn record(&mut self, bytes: usize, name: &str) {
        const WINDOW_MILLIS: u64 = Duration::from_secs(5).as_millis() as u64;
        let now =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let start_time = self.window_start;

        if start_time == 0 {
            self.window_start = now;
            self.window_bytes = bytes as u64;
        } else if now.saturating_sub(start_time) < WINDOW_MILLIS {
            self.window_bytes += bytes as u64;
        } else {
            let window_bytes = self.window_bytes;
            let elapsed = now.saturating_sub(start_time) as f64 / 1000.0;
            let current_throughput = (window_bytes as f64 / elapsed) as u64;
            debug!(
                "{} Throughput: {:.1} Mbits/sec",
                name,
                WifiStats::bytes_ps_to_mbps(current_throughput)
            );
            if current_throughput > self.max_throughput {
                self.max_throughput = current_throughput;
            }
            self.window_start = now;
            self.window_bytes = bytes as u64;
        }
    }
}
