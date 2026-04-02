// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{sync::mpsc, time::Duration};

use bytes::Bytes;

use crate::types::ModemSink;

/// Mock handler for modem callbacks that captures responses.
pub struct MockModemHandler {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl MockModemHandler {
    pub fn new() -> (Self, ModemSink) {
        let (tx, rx) = mpsc::channel();
        let sink =
            ModemSink::new(move |item: Bytes| tx.send(item.to_vec()).map_err(|e| e.to_string()));
        (Self { rx }, sink)
    }

    /// Waits for a response.
    /// Since the simulator is synchronous, responses should be available
    /// immediately.
    pub fn wait_for_response(&mut self) -> Vec<u8> {
        // Use recv_timeout to avoid hanging forever if logic is wrong, but typically
        // it's instant.
        self.rx
            .recv_timeout(Duration::from_secs(1))
            .expect("Test timed out waiting for response (or channel closed)")
    }

    /// Checks if a response is available (non-blocking).
    pub fn try_get_response(&mut self) -> Option<Vec<u8>> {
        self.rx.try_recv().ok()
    }
}
