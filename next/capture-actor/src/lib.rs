// Copyright 2025 The Android Open Source Project

//! Capture Actor
//!
//! This crate provides an actor for managing packet captures. It handles
//! starting and stopping captures, writing to PCAP files, and managing
//! capture state for different chips.

mod bt_pcap;
mod capture_actor;
mod error;
mod lifecycle;
mod service;
mod uwb_pcap;
mod wifi_pcap;
mod writer;

use actor_framework::ResourceActor;
pub use capture_actor::CaptureActor;
pub use error::CaptureError;

pub mod client;
pub use client::CaptureClient;

/// Creates a new Capture actor and its client.
pub fn new() -> (ResourceActor<CaptureActor>, CaptureClient) {
    // Buffer size of 32 is sufficient for capture control commands.
    // Packet data flows through a separate channel if needed, but here we handle
    // control.
    let (runner, client) = ResourceActor::new(32);
    (runner, CaptureClient::new(client))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
        time::SystemTime,
    };

    use capture_api::Direction;

    use crate::bt_pcap::BluetoothH4Writer;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[tokio::test]
    async fn test_pcap_writer() {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("netsim_test_pcap_writer_{}", id));
        fs::create_dir_all(&dir).unwrap();
        let filename = dir.join("test_pcap.pcap");
        let mut writer = BluetoothH4Writer::new(&filename).await.unwrap();
        let data = vec![0x01, 0x00, 0x00, 0x00]; // Fake H4 Command
        writer.write_packet(SystemTime::now(), Direction::Sent, &data).await.unwrap();

        let (records, bytes) = writer.get_stats();
        assert_eq!(records, 1);
        assert_eq!(bytes, 4);

        // explicitly drop writer to ensure file handle closed (though not strictly
        // required for remove_file on linux)
        drop(writer);
        fs::remove_file(filename).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
