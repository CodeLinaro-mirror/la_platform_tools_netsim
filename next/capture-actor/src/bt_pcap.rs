// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Bluetooth H4 PCAP Writer
//!
//! This module provides a PCAP writer specifically for Bluetooth H4 packets.

use std::{io::Result, path::Path};

use crate::writer::{CaptureWriter, DLT_BLUETOOTH_H4, PcapWriter};

/// PCAP writer for Bluetooth H4 packets.
pub struct BluetoothH4Writer;

impl BluetoothH4Writer {
    /// Creates a new Bluetooth H4 PCAP writer.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(PcapWriter::new(path, DLT_BLUETOOTH_H4).await?))
    }
}
