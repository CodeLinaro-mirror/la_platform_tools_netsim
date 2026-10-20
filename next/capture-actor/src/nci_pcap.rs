// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{io, path::Path};

use crate::writer::{CaptureWriter, DLT_NFC_NCI, PcapWriter};

/// PCAP writer for NFC NCI packets.
pub struct NciPcapWriter;

impl NciPcapWriter {
    /// Wraps [`PcapWriter`] with [`DLT_NFC_NCI`] as the format.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new<P: AsRef<Path>>(path: P) -> io::Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(PcapWriter::new(path, DLT_NFC_NCI).await?))
    }
}
