// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Capture Writers
//!
//! This module provides traits and implementations for writing packet captures
//! to files. Currently supports PCAP format with Bluetooth H4 encapsulation.

use std::{io, path::Path, time::SystemTime};

use async_trait::async_trait;
use capture_api::Direction;
use netsim_packets::{PcapHeader, PcapRecordHeader};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};
use zerocopy::{I32, IntoBytes, U16, U32};

/// Trait for writing packet captures.
///
/// Implementations of this trait handle the actual writing of packets to a
/// specific format.
#[async_trait]
pub trait CaptureWriter: Send + Sync {
    /// Writes a packet to the capture.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The time the packet was captured.
    /// * `direction` - The direction of the packet (Sent/Received).
    /// * `data` - The packet data.
    async fn write_packet(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        data: &[u8],
    ) -> io::Result<()>;

    async fn flush(&mut self) -> io::Result<()>;

    /// Returns statistics about the capture.
    ///
    /// Returns a tuple of (number of records, total bytes written).
    fn get_stats(&self) -> (u64, u64); // (records, bytes)
}

/// Generic PCAP writer.
///
/// Handles writing the PCAP global header and record headers.
pub struct PcapWriter {
    writer: BufWriter<File>,
    records_written: u64,
    bytes_written: u64,
}

impl PcapWriter {
    /// Creates a new PCAP writer.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the output file.
    /// * `network` - The Link-Layer Header Type (DLT).
    pub async fn new<P: AsRef<Path>>(path: P, network: u32) -> io::Result<Self> {
        let file = File::create(path).await?;
        let mut writer = BufWriter::new(file);

        let header = PcapHeader {
            magic_number: U32::new(0xa1b2c3d4),
            version_major: U16::new(2),
            version_minor: U16::new(4),
            thiszone: I32::new(0),
            sigfigs: U32::new(0),
            snaplen: U32::new(65535),
            network: U32::new(network),
        };

        writer.write_all(header.as_bytes()).await?;

        Ok(Self { writer, records_written: 0, bytes_written: 0 })
    }
}

#[async_trait]
impl CaptureWriter for PcapWriter {
    async fn write_packet(
        &mut self,
        timestamp: SystemTime,
        _direction: Direction,
        data: &[u8],
    ) -> io::Result<()> {
        let duration = timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let ts_sec = duration.as_secs() as u32;
        let ts_usec = duration.subsec_micros() as u32;

        let header = PcapRecordHeader {
            ts_sec: U32::new(ts_sec),
            ts_usec: U32::new(ts_usec),
            incl_len: U32::new(data.len() as u32),
            orig_len: U32::new(data.len() as u32),
        };

        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(data).await?;

        self.records_written += 1;
        self.bytes_written += data.len() as u64;

        Ok(())
    }

    async fn flush(&mut self) -> io::Result<()> {
        self.writer.flush().await
    }

    fn get_stats(&self) -> (u64, u64) {
        (self.records_written, self.bytes_written)
    }
}

// Bluetooth H4 DLT is 187
pub const DLT_BLUETOOTH_H4: u32 = 187;
// FiRa UCI DLT is 299
pub const DLT_FIRA_UCI: u32 = 299;
// IEEE 802.11 Radiotap DLT is 127
pub const DLT_IEEE802_11_RADIO: u32 = 127;
// Ethernet DLT is 1
pub const DLT_ETHERNET: u32 = 1;
// NFC Controller Interface (NCI) uses DLT 147 (USER0) for Wireshark support
pub const DLT_NFC_NCI: u32 = 147;
// User defined 0 DLT is 147 (for custom protocols like AT commands)
pub const DLT_USER0: u32 = 147;
