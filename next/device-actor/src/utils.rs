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

#[derive(Debug)]
pub struct StreamStats {
    pub tx_packets: AtomicU64,
    pub rx_packets: AtomicU64,
    pub tx_bytes: AtomicU64,
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

            let capture_callback: Box<dyn Fn(ChipId, capture_api::Direction, Bytes) + Send + Sync> =
                if let Some(client) = &capture_client {
                    let cc_clone = client.clone();
                    Box::new(move |id, dir, bytes| {
                        cc_clone.capture_packet(id, dir, bytes);
                    })
                } else {
                    Box::new(|_, _, _| {})
                };

            let capture_callback2: Box<
                dyn Fn(ChipId, capture_api::Direction, Bytes) + Send + Sync,
            > = if let Some(client) = &capture_client {
                let cc_clone = client.clone();
                Box::new(move |id, dir, bytes| {
                    cc_clone.capture_packet(id, dir, bytes);
                })
            } else {
                Box::new(|_, _, _| {})
            };

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
        (s, k) => (s, k, None),
    }
}
