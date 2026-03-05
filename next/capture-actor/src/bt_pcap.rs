//! # Bluetooth H4 PCAP Writer
//!
//! This module provides a PCAP writer specifically for Bluetooth H4 packets.

use std::{path::Path, time::SystemTime};

use anyhow::Result;
use capture_api::Direction;

use crate::writer::{CaptureWriter, PcapWriter, DLT_BLUETOOTH_H4};

/// PCAP writer for Bluetooth H4 packets.
pub struct BluetoothH4Writer {
    pcap_writer: PcapWriter,
}

impl BluetoothH4Writer {
    /// Creates a new Bluetooth H4 PCAP writer.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self { pcap_writer: PcapWriter::new(path, DLT_BLUETOOTH_H4)? })
    }
}

impl CaptureWriter for BluetoothH4Writer {
    fn write_packet(
        &mut self,
        timestamp: SystemTime,
        direction: Direction,
        data: &[u8],
    ) -> Result<()> {
        // For H4, we expect the data to already contain the H4 type byte if it's a
        // standard H4 packet. If Netsim provides raw HCI packets, we might need
        // to prepend the type byte based on direction and packet type. Assuming
        // for now that 'data' is the full H4 packet or we just write it as is.
        // If we need to handle direction specifically for H4 (which doesn't support it
        // natively in the DLT, it relies on the H4 type byte), we just pass it
        // through. If the user wants to encode direction, they should use a
        // different DLT or ensure H4 types are correct.
        self.pcap_writer.write_packet(timestamp, direction, data)
    }

    fn get_stats(&self) -> (u64, u64) {
        self.pcap_writer.get_stats()
    }
}
