// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Ethernet PCAP Writer
//!
//! This module provides a PCAP writer specifically for Ethernet packets.

use std::{io, path::Path};

use crate::writer::{CaptureWriter, DLT_ETHERNET, PcapWriter};

/// PCAP writer for Ethernet packets.
pub struct EthernetPcapWriter;

impl EthernetPcapWriter {
    /// Wraps [`PcapWriter`] with [`DLT_ETHERNET`] as the format.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new<P: AsRef<Path>>(path: P) -> io::Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(PcapWriter::new(path, DLT_ETHERNET).await?))
    }
}
