//! # Device Actor Utilities
//!
//! This module provides utility functions for the device actor,
//! including network kind conversion and capture stream wrapping.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use bytes::Bytes;
use capture_api::{
    io::{CapturedSink, CapturedStream},
    CaptureCreate, CaptureSender,
};
use futures::{SinkExt, StreamExt};
use netsim_model::{
    chip::{PacketSink, PacketStream},
    ChipId, ChipKind,
};
use netsim_proto::protobuf::Enum;

#[derive(Debug)]
pub struct StreamStats {
    /// Total packets transmitted by the chip (RX from transport perspective)
    pub tx_packets: AtomicU64,
    /// Total packets received by the chip (TX from transport perspective)
    pub rx_packets: AtomicU64,
    /// Total bytes transmitted by the chip
    pub tx_bytes: AtomicU64,
    /// Total bytes received by the chip
    pub rx_bytes: AtomicU64,
    pub start_time: Instant,
}

impl PartialEq for StreamStats {
    fn eq(&self, other: &Self) -> bool {
        self.tx_packets.load(Ordering::Relaxed) == other.tx_packets.load(Ordering::Relaxed)
            && self.rx_packets.load(Ordering::Relaxed) == other.rx_packets.load(Ordering::Relaxed)
            && self.tx_bytes.load(Ordering::Relaxed) == other.tx_bytes.load(Ordering::Relaxed)
            && self.rx_bytes.load(Ordering::Relaxed) == other.rx_bytes.load(Ordering::Relaxed)
            && self.start_time == other.start_time
    }
}

impl Default for StreamStats {
    fn default() -> Self {
        Self {
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            tx_bytes: AtomicU64::new(0),
            rx_bytes: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
}

pub async fn create_capture_and_wrap_streams(
    capture_client: Option<Arc<dyn CaptureSender>>,
    chip_id: ChipId,
    chip_kind: ChipKind,
    device_name: String,
    packet_stream: Option<PacketStream>,
    packet_sink: Option<PacketSink>,
) -> (Option<PacketStream>, Option<PacketSink>, Option<Arc<StreamStats>>) {
    let enabled_flag = Arc::new(AtomicBool::new(false));

    if let Some(client) = &capture_client {
        let capture_create = CaptureCreate {
            chip_id,
            chip_kind,
            device_name: device_name.to_string(),
            default_enabled: false,
            enabled_flag: enabled_flag.clone(),
        };
        let _ = client.create_capture(capture_create).await;
    }

    match (packet_stream, packet_sink) {
        (Some(in_stream), Some(in_sink)) => {
            let stats = Arc::new(StreamStats::default());
            let stats_clone_rx = stats.clone();
            let stats_clone_tx = stats.clone();

            let create_callback =
                || -> Box<dyn Fn(ChipId, capture_api::Direction, Bytes) + Send + Sync> {
                    if let Some(client) = &capture_client {
                        let cc_clone = client.clone();
                        Box::new(move |id, dir, bytes| {
                            cc_clone.capture_packet(id, dir, bytes);
                        })
                    } else {
                        Box::new(|_, _, _| {})
                    }
                };

            let capture_callback = create_callback();
            let capture_callback2 = create_callback();

            // Wrap stream for stats (Rx from Transport perspective)
            let in_stream = in_stream.inspect(move |bytes| {
                stats_clone_rx.rx_packets.fetch_add(1, Ordering::Relaxed);
                stats_clone_rx.rx_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
            });

            // Wrap sink for stats (Tx to Transport perspective)
            let in_sink = in_sink.with(move |bytes: Bytes| {
                stats_clone_tx.tx_packets.fetch_add(1, Ordering::Relaxed);
                stats_clone_tx.tx_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                futures::future::ready(Ok(bytes))
            });

            let (wrapped_stream, wrapped_sink) = (
                CapturedStream::new(
                    Box::new(in_stream),
                    capture_callback,
                    chip_id,
                    enabled_flag.clone(),
                ),
                CapturedSink::new(Box::pin(in_sink), capture_callback2, chip_id, enabled_flag),
            );

            let out_stream: PacketStream =
                Box::new(wrapped_stream.filter_map(|item| futures::future::ready(Some(item))));

            let out_sink: PacketSink = Box::pin(
                wrapped_sink.sink_map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
            );

            (Some(out_stream), Some(out_sink), Some(stats))
        }
        (s, k) => (s, k, Some(Arc::new(StreamStats::default()))),
    }
}

/// Merges StreamStats (atomic counters) into generic RadioStats if the client
/// reported zero values (which often happens when the client is just a
/// wrapper or crashed).
/// Distributes StreamStats (bytes/counts) across a list of RadioStats.
///
/// If the actor reports counts, we use those to distribute the total bytes from
/// StreamStats proportional to the counts. We do NOT overwrite the actor's
/// counts in this case.
///
/// If the actor reports NO counts (total = 0), we fallback to using StreamStats
/// for both counts and bytes, distributing them evenly (or to the first entry)
/// as a best-effort.
pub fn distribute_stream_stats(
    radio_stats_list: &mut [netsim_model::stats::NetsimRadioStats],
    stream_stats: &StreamStats,
) {
    if radio_stats_list.is_empty() {
        return;
    }

    let total_tx_count: u64 = radio_stats_list.iter().map(|s| s.tx_count).sum();
    let total_rx_count: u64 = radio_stats_list.iter().map(|s| s.rx_count).sum();

    // Stream RX is Chip TX
    let stream_tx_bytes = std::cmp::min(stream_stats.rx_bytes.load(Ordering::Relaxed), u64::MAX);
    let stream_tx_count = std::cmp::min(stream_stats.rx_packets.load(Ordering::Relaxed), u64::MAX);
    // Stream TX is Chip RX
    let stream_rx_bytes = std::cmp::min(stream_stats.tx_bytes.load(Ordering::Relaxed), u64::MAX);
    let stream_rx_count = std::cmp::min(stream_stats.tx_packets.load(Ordering::Relaxed), u64::MAX);

    if total_tx_count > 0 {
        // Distribute TX bytes proportional to TX counts
        for stat in radio_stats_list.iter_mut() {
            if stat.tx_bytes == 0 {
                stat.tx_bytes = (stream_tx_bytes as u128 * stat.tx_count as u128
                    / total_tx_count as u128) as u64;
            }
        }
    } else if stream_tx_count > 0 {
        // Fallback: Actor reports 0 counts, but Stream sees traffic.
        // Assign all to the first entry (or we could split, but first is safer
        // default).
        if let Some(first) = radio_stats_list.first_mut() {
            if first.tx_bytes == 0 {
                first.tx_bytes = stream_tx_bytes;
            }
            if first.tx_count == 0 {
                first.tx_count = stream_tx_count;
            }
        }
    }

    if total_rx_count > 0 {
        // Distribute RX bytes proportional to RX counts
        for stat in radio_stats_list.iter_mut() {
            if stat.rx_bytes == 0 {
                stat.rx_bytes = (stream_rx_bytes as u128 * stat.rx_count as u128
                    / total_rx_count as u128) as u64;
            }
        }
    } else if stream_rx_count > 0 {
        // Fallback: Actor reports 0 counts, but Stream sees traffic.
        if let Some(first) = radio_stats_list.first_mut() {
            if first.rx_bytes == 0 {
                first.rx_bytes = stream_rx_bytes;
            }
            if first.rx_count == 0 {
                first.rx_count = stream_rx_count;
            }
        }
    }
}

/// Helper to saturate u64 to i32::MAX
fn saturate_cast(val: u64) -> i32 {
    std::cmp::min(val, i32::MAX as u64) as i32
}

/// Converts internal Model stats to Proto stats for persistence/RPC.
pub fn to_proto_stats(
    m: netsim_model::stats::NetsimRadioStats,
) -> netsim_proto::stats::NetsimRadioStats {
    let mut p = netsim_proto::stats::NetsimRadioStats::new();
    p.set_device_id(m.id);
    // Convert RadioKind to i32 for proto
    let k_i32: i32 = m.kind.into();
    if let Some(k) = netsim_proto::stats::netsim_radio_stats::Kind::from_i32(k_i32) {
        p.set_kind(k);
    } else {
        p.set_kind(netsim_proto::stats::netsim_radio_stats::Kind::UNSPECIFIED);
    }
    p.set_duration_secs(m.duration_secs);
    p.set_tx_count(saturate_cast(m.tx_count));
    p.set_rx_count(saturate_cast(m.rx_count));
    p.set_tx_bytes(saturate_cast(m.tx_bytes));
    p.set_rx_bytes(saturate_cast(m.rx_bytes));
    // invalid_packets are not yet persisted/converted
    p
}

/// Creates a Proto RadioStats object directly from StreamStats.
/// Used for fallback when the actor is not reporting stats.
pub fn stream_to_proto_stats(
    device_id: u32,
    kind: netsim_model::stats::RadioKind,
    duration_secs: u64,
    stream_stats: &StreamStats,
) -> netsim_proto::stats::NetsimRadioStats {
    let mut p = netsim_proto::stats::NetsimRadioStats::new();
    p.set_device_id(device_id);
    let k_i32: i32 = kind.into();
    if let Some(k) = netsim_proto::stats::netsim_radio_stats::Kind::from_i32(k_i32) {
        p.set_kind(k);
    } else {
        p.set_kind(netsim_proto::stats::netsim_radio_stats::Kind::UNSPECIFIED);
    }
    p.set_duration_secs(duration_secs);

    // Stream RX is Chip TX
    p.set_tx_count(saturate_cast(stream_stats.rx_packets.load(Ordering::Relaxed)));
    p.set_tx_bytes(saturate_cast(stream_stats.rx_bytes.load(Ordering::Relaxed)));
    // Stream TX is Chip RX
    p.set_rx_count(saturate_cast(stream_stats.tx_packets.load(Ordering::Relaxed)));
    p.set_rx_bytes(saturate_cast(stream_stats.tx_bytes.load(Ordering::Relaxed)));

    p
}

/// Creates a Model RadioStats object directly from StreamStats.
/// Used for fallback when the actor is not reporting stats.
pub fn stream_to_model_stats(
    device_id: u32,
    kind: netsim_model::stats::RadioKind,
    duration_secs: u64,
    stream_stats: &StreamStats,
) -> netsim_model::stats::NetsimRadioStats {
    netsim_model::stats::NetsimRadioStats {
        id: device_id,
        name: "".to_string(), // or derive from kind?
        kind,
        duration_secs,
        tx_count: stream_stats.rx_packets.load(Ordering::Relaxed),
        rx_count: stream_stats.tx_packets.load(Ordering::Relaxed),
        tx_bytes: stream_stats.rx_bytes.load(Ordering::Relaxed),
        rx_bytes: stream_stats.tx_bytes.load(Ordering::Relaxed),
        invalid_packets: vec![],
    }
}

pub fn to_proto_device_stats(
    device_id: u32,
    info: &netsim_model::device::DeviceInfo,
) -> netsim_proto::stats::NetsimDeviceStats {
    let mut stats = netsim_proto::stats::NetsimDeviceStats::new();
    stats.set_device_id(device_id);
    stats.set_kind(info.kind.clone());
    stats.set_version(info.version.clone());
    stats.set_sdk_version(info.sdk_version.clone());
    stats.set_build_id(info.build_id.clone());
    stats.set_variant(info.variant.clone());
    stats.set_arch(info.arch.clone());
    stats
}
