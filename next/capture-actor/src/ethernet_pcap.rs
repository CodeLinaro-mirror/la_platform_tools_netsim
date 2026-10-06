// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # Ethernet PCAP Writer
//!
//! This module provides a PCAP writer specifically for Ethernet packets.

use std::{io, path::Path};

use crate::writer::{CaptureWriter, DataLinkType, PcapWriter};

/// PCAP writer for Ethernet packets.
pub(crate) struct EthernetPcapWriter;

impl EthernetPcapWriter {
    /// Wraps [`PcapWriter`] with [`DataLinkType::Ethernet`] as the format.
    #[allow(clippy::new_ret_no_self)]
    pub(crate) async fn new<P: AsRef<Path>>(path: P) -> io::Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(PcapWriter::new(path, DataLinkType::Ethernet).await?))
    }
}
