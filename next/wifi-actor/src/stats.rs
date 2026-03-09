// Copyright 2025 Google LLC

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{debug, warn};

use crate::error::WifiError;

pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now_millis(&self) -> u64;
}

#[derive(Debug, Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }
}

#[derive(Debug, Clone)]
pub struct MockClock {
    time_millis: Arc<std::sync::atomic::AtomicU64>,
}

impl MockClock {
    pub fn new() -> Self {
        Self { time_millis: Arc::new(std::sync::atomic::AtomicU64::new(0)) }
    }

    pub fn store(&self, time: u64) {
        self.time_millis.store(time, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Clock for MockClock {
    fn now_millis(&self) -> u64 {
        self.time_millis.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Stats for the Wifi Actor.
///
/// Tracks error counts, packet counts, and throughput metrics.
/// Designed for single-threaded usage within the actor (no internal locking).
#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct ThroughputValues {
    clock: Arc<dyn Clock>,
    max_throughput: u64,
    window_start: u64,
    window_bytes: u64,
}

impl ThroughputValues {
    fn new(clock: Arc<dyn Clock>) -> Self {
        Self { clock, max_throughput: 0, window_start: 0, window_bytes: 0 }
    }
}

#[derive(Clone, Debug)]
struct WifiValues {
    download_throughput: ThroughputValues,
    upload_throughput: ThroughputValues,
}

impl WifiValues {
    fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            download_throughput: ThroughputValues::new(clock.clone()),
            upload_throughput: ThroughputValues::new(clock),
        }
    }
}

impl WifiStats {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self { counts: Default::default(), values: WifiValues::new(clock) }
    }
    pub fn record_download_bytes(&mut self, bytes: usize) {
        self.values.download_throughput.record(bytes, "Download");
    }

    pub fn record_upload_bytes(&mut self, bytes: usize) {
        self.values.upload_throughput.record(bytes, "Upload");
    }

    pub fn log_and_incr_err_count(&mut self, error: &WifiError) {
        warn!("{error}");
        match error {
            WifiError::Hostapd(_) => self.counts.hostapd_error += 1,
            WifiError::Network(_) => self.counts.network_error += 1,
            WifiError::Client(_) => self.counts.client_error += 1,
            WifiError::Frame(_) => self.counts.frame_error += 1,
            WifiError::Transmission(_) => self.counts.transmission_error += 1,
            WifiError::Other(_) => self.counts.other_error += 1,
            WifiError::Internal(_) => self.counts.other_error += 1,
        }
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

    pub fn to_proto(&self) -> netsim_proto::stats::WifiStats {
        let mut proto = netsim_proto::stats::WifiStats::new();

        let saturate = |val: u64| -> i32 { std::cmp::min(val, i32::MAX as u64) as i32 };

        proto.set_hostapd_errors(saturate(self.counts.hostapd_error));
        proto.set_network_errors(saturate(self.counts.network_error));
        proto.set_client_errors(saturate(self.counts.client_error));
        proto.set_frame_errors(saturate(self.counts.frame_error));
        proto.set_transmission_errors(saturate(self.counts.transmission_error));
        proto.set_other_errors(saturate(self.counts.other_error));

        proto.set_hwsim_frames_rx(saturate(self.counts.hwsim_frames_rx));
        proto.set_hwsim_frames_tx(saturate(self.counts.hwsim_frames_tx));
        proto.set_network_packets_tx(saturate(self.counts.network_packets_tx));
        proto.set_network_packets_rx(saturate(self.counts.network_packets_rx));
        proto.set_hostapd_frames_tx(saturate(self.counts.hostapd_frames_tx));
        proto.set_hostapd_frames_rx(saturate(self.counts.hostapd_frames_rx));
        proto.set_wmedium_frames_tx(saturate(self.counts.wmedium_frames_tx));
        proto.set_wmedium_unicast_frames_tx(saturate(self.counts.wmedium_unicast_frames_tx));
        proto.set_mgmt_frames_rx(saturate(self.counts.mgmt_frames_rx));
        proto.set_mdns_count(saturate(self.counts.mdns_count));

        proto.set_max_download_throughput(Self::bytes_ps_to_mbps(
            self.values.download_throughput.max_throughput,
        ));
        proto.set_max_upload_throughput(Self::bytes_ps_to_mbps(
            self.values.upload_throughput.max_throughput,
        ));

        proto
    }
}

impl ThroughputValues {
    fn record(&mut self, bytes: usize, name: &str) {
        const WINDOW_MILLIS: u64 = Duration::from_secs(5).as_millis() as u64;
        let now = self.clock.now_millis();
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
