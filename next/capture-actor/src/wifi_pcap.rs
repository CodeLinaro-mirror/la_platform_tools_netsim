// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Wi-Fi PCAP Writer
//!
//! This module provides a PCAP writer specifically for Wi-Fi packets.
//! It implements the Radiotap header encapsulation for IEEE 802.11 payloads.

use std::{
    io::{self, Result},
    path::Path,
    time::SystemTime,
};

use async_trait::async_trait;
use capture_api::Direction;
use netsim_packets::{HwsimCmd, HwsimFrame, HwsimMsg, create_radiotap_packet};
use tracing::warn;

use crate::writer::{CaptureWriter, DLT_IEEE802_11_RADIO, PcapWriter};

/// PCAP writer for Wi-Fi packets.
///
/// Wraps IEEE 802.11 packets with a Radiotap header.
pub struct WifiPcapWriter {
    inner: PcapWriter,
}

impl WifiPcapWriter {
    /// Creates a new Wi-Fi PCAP writer.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(Self { inner: PcapWriter::new(path, DLT_IEEE802_11_RADIO).await? }))
    }
}

#[async_trait]
impl CaptureWriter for WifiPcapWriter {
    async fn write_packet(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        data: &[u8],
    ) -> io::Result<()> {
        match HwsimMsg::decode_full(data) {
            Ok(hwsim_msg) => {
                if hwsim_msg.hwsim_hdr.hwsim_cmd != HwsimCmd::Frame {
                    return Ok(());
                }
                match HwsimFrame::parse(&hwsim_msg) {
                    Ok(frame) => {
                        let radiotap_packet = create_radiotap_packet(&frame);
                        self.inner.write_packet(timestamp, direction, &radiotap_packet).await
                    }
                    Err(e) => {
                        warn!("WifiPcapWriter: Failed to parse HwsimFrame from HwsimMsg: {:?}", e);
                        Ok(())
                    }
                }
            }
            Err(e) => {
                warn!("WifiPcapWriter: Failed to decode HwsimMsg: {:?}", e);
                Ok(())
            }
        }
    }

    async fn flush(&mut self) -> Result<()> {
        self.inner.flush().await
    }

    fn get_stats(&self) -> (u64, u64) {
        self.inner.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[tokio::test]
    async fn test_wifi_pcap_writer() {
        let dir = std::env::temp_dir();
        let filepath = dir.join("test_wifi_netsim.pcap");
        let _ = fs::remove_file(&filepath); // Clean up any old file

        let mut writer = WifiPcapWriter::new(&filepath).await.unwrap();
        // Since `[1,2,3]` is an invalid HwsimMsg, it will fail to decode and write 0
        // packets.
        writer.write_packet(SystemTime::now(), Direction::Sent, &[1, 2, 3]).await.unwrap();

        let (records, bytes) = writer.get_stats();
        assert_eq!(records, 0);
        assert_eq!(bytes, 0);
    }

    #[tokio::test]
    async fn test_wifi_pcap_writer_filter_tx_info() {
        let dir = std::env::temp_dir();
        let filepath = dir.join("test_wifi_filter_tx_info.pcap");
        let _ = fs::remove_file(&filepath);

        let mut writer = WifiPcapWriter::new(&filepath).await.unwrap();

        // Construct a HwsimMsg with HwsimCmd::TxInfoFrame
        let mut data = vec![0u8; 20];
        data[0..16].copy_from_slice(&[0; 16]); // NlMsgHdr
        data[16] = 3; // HwsimCmd::TxInfoFrame
        data[17] = 1; // HwsimMsgHdr version

        writer.write_packet(SystemTime::now(), Direction::Sent, &data).await.unwrap();

        let (records, bytes) = writer.get_stats();
        assert_eq!(records, 0);
        assert_eq!(bytes, 0);
    }
}
