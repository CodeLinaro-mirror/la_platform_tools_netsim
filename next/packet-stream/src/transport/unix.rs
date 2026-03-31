// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0
//=============================================================================
// src/socket/unix.rs - Unix Domain Socket implementation
//=============================================================================

use std::path::Path;

use tokio::net::{UnixListener, UnixStream};

use crate::error::Result;

pub struct UnixSocketListener {
    listener: UnixListener,
    path: std::path::PathBuf,
}

impl UnixSocketListener {
    pub async fn bind<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_owned();

        // Remove existing socket file if it exists
        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;

        Ok(Self { listener, path })
    }

    pub async fn accept(&self) -> Result<UnixStream> {
        let (stream, _) = self.listener.accept().await?;
        Ok(stream)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for UnixSocketListener {
    fn drop(&mut self) {
        // Clean up socket file on drop
        let _ = std::fs::remove_file(&self.path);
    }
}

pub async fn connect_unix<P: AsRef<Path>>(path: P) -> Result<UnixStream> {
    let stream = UnixStream::connect(path).await?;
    Ok(stream)
}
