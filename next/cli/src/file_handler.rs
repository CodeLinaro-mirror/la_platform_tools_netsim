// Copyright 2022 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    error::{Error, Result},
    grpc_client::ClientResponseReadable,
};

pub(crate) struct FileHandler {
    pub file: File,
    pub path: PathBuf,
}

impl ClientResponseReadable for FileHandler {
    fn handle_chunk(&mut self, chunk: &[u8]) -> Result<()> {
        (&self.file).write_all(chunk).map_err(|e| {
            Error::from(format!("Unable to write to file {}: {e}", self.path.display()))
        })
    }
}
