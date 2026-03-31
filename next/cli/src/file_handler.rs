// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

/// Implements handler for pcap operations
use std::io::Write;
use std::{fs::File, path::PathBuf};

use crate::grpc_client::ClientResponseReadable;

pub struct FileHandler {
    pub file: File,
    pub path: PathBuf,
}

impl ClientResponseReadable for FileHandler {
    // function to handle writing each chunk to file
    fn handle_chunk(&self, chunk: &[u8]) {
        (&self.file)
            .write_all(chunk)
            .unwrap_or_else(|_| panic!("Unable to write to file: {}", self.path.display()));
    }
}
