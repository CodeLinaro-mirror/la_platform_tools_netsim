//! # Device Actor Utilities
//!
//! This module provides utility functions for the device actor,
//! including network kind conversion and capture stream wrapping.

use capture_api::io::{CapturedSink, CapturedStream};
use capture_api::{CaptureCreate, CaptureSender};
use futures::{SinkExt, StreamExt};
use netsim_model::chip::{ChipId, PacketSink, PacketStream};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn chip_kind_to_network_kind(
    kind: &netsim_model::chip::ChipKind,
) -> netsim_model::chip::NetworkKind {
    match kind {
        netsim_model::chip::ChipKind::BLUETOOTH | netsim_model::chip::ChipKind::BleBeacon => {
            netsim_model::chip::NetworkKind::Bluetooth
        }
        netsim_model::chip::ChipKind::WIFI => netsim_model::chip::NetworkKind::Wifi,
        netsim_model::chip::ChipKind::UWB => netsim_model::chip::NetworkKind::Uwb,
        netsim_model::chip::ChipKind::CELLULAR => netsim_model::chip::NetworkKind::Cell,
        _ => netsim_model::chip::NetworkKind::Bluetooth, // Fallback
    }
}

pub async fn create_capture_and_wrap_streams(
    capture_client: Arc<dyn CaptureSender>,
    chip_id: ChipId,
    chip_kind: netsim_model::chip::NetworkKind,
    device_name: String,
    packet_stream: Option<PacketStream>,
    packet_sink: Option<PacketSink>,
) -> (Option<PacketStream>, Option<PacketSink>) {
    let enabled_flag = Arc::new(AtomicBool::new(false));
    let capture_create = CaptureCreate {
        chip_id,
        chip_kind: chip_kind.into(),
        device_name: device_name.to_string(),
        default_enabled: false, // Default to disabled
        enabled_flag: Arc::new(AtomicBool::new(false)),
    };

    let _ = capture_client.create_capture(capture_create).await;

    if let (Some(in_stream), Some(in_sink)) = (packet_stream, packet_sink) {
        let cc_clone = capture_client.clone();
        let capture_callback = Box::new(move |id, dir, bytes| {
            cc_clone.capture_packet(id, dir, bytes);
        });
        let cc_clone2 = capture_client.clone();
        let capture_callback2 = Box::new(move |id, dir, bytes| {
            cc_clone2.capture_packet(id, dir, bytes);
        });

        let (wrapped_stream, wrapped_sink) = (
            CapturedStream::new(in_stream, capture_callback, chip_id, enabled_flag.clone()),
            CapturedSink::new(in_sink, capture_callback2, chip_id, enabled_flag),
        );

        let out_stream: PacketStream =
            Box::new(wrapped_stream.filter_map(|item| futures::future::ready(Some(item))));

        let out_sink: PacketSink = Box::pin(
            wrapped_sink.sink_map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );

        (Some(out_stream), Some(out_sink))
    } else {
        (None, None)
    }
}
