//! # Capture Writers
//!
//! This module provides traits and implementations for writing packet captures
//! to files. Currently supports PCAP format with Bluetooth H4 encapsulation.

use std::{io, path::Path, time::SystemTime};

use anyhow::Result;
use async_trait::async_trait;
use capture_api::Direction;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

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
    ) -> Result<()>;

    /// Returns statistics about the capture.
    ///
    /// Returns a tuple of (number of records, total bytes written).
    fn get_stats(&self) -> (u64, u64); // (records, bytes)
}

#[derive(IntoBytes, FromBytes, Immutable, Copy, Clone, Debug)]
#[repr(C)]
struct PcapGlobalHeader {
    magic_number: u32,
    version_major: u16,
    version_minor: u16,
    thiszone: i32,
    sigfigs: u32,
    snaplen: u32,
    network: u32,
}

#[derive(IntoBytes, FromBytes, Immutable, Copy, Clone, Debug)]
#[repr(C)]
struct PcapRecordHeader {
    ts_sec: u32,
    ts_usec: u32,
    incl_len: u32,
    orig_len: u32,
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

        let header = PcapGlobalHeader {
            magic_number: 0xa1b2c3d4,
            version_major: 2,
            version_minor: 4,
            thiszone: 0,
            sigfigs: 0,
            snaplen: 65535,
            network,
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
    ) -> Result<()> {
        let duration = timestamp.duration_since(SystemTime::UNIX_EPOCH)?;
        let ts_sec = duration.as_secs() as u32;
        let ts_usec = duration.subsec_micros() as u32;

        let header = PcapRecordHeader {
            ts_sec,
            ts_usec,
            incl_len: data.len() as u32,
            orig_len: data.len() as u32,
        };

        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(data).await?;

        self.records_written += 1;
        self.bytes_written += data.len() as u64;

        Ok(())
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
