// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{io, path::Path};

use crate::writer::{CaptureWriter, DataLinkType, PcapWriter};

/// PCAP writer for NFC NCI packets.
pub(crate) struct NciPcapWriter;

impl NciPcapWriter {
    /// Wraps [`PcapWriter`] with [`DataLinkType::User0`] as the format.
    #[allow(clippy::new_ret_no_self)]
    pub(crate) async fn new<P: AsRef<Path>>(path: P) -> io::Result<Box<dyn CaptureWriter>> {
        Ok(Box::new(PcapWriter::new(path, DataLinkType::User0).await?))
    }
}
