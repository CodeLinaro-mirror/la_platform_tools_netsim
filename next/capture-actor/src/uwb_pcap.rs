// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{io, path::Path};

use crate::writer::{CaptureWriter, PcapWriter, DLT_FIRA_UCI};

/// PCAP writer for UWB UCI packets.
pub struct UwbPcapWriter;

impl UwbPcapWriter {
    /// Wraps [`PcapWriter`] with [`DLT_FIRA_UCI`] as the format.
    pub async fn new<P: AsRef<Path>>(path: P) -> io::Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(PcapWriter::new(path, DLT_FIRA_UCI).await?))
    }
}
