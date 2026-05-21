// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::SystemTime,
};

use actor_framework::FrameworkError;
use bytes::Bytes;
use capture_actor::{CaptureActor, CaptureClient, CaptureError};
use capture_api::{CaptureCreate, CaptureInfo, CaptureSender, Direction};
use netsim_model::{ChipId, ChipKind, ClientError};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The BDD World for Capture Actor tests.
pub struct World {
    pub client: CaptureClient,
    pub temp_dir: PathBuf,
    pub keep_temp_dir: bool,
    _actor_task: tokio::task::JoinHandle<()>,
}

impl Drop for World {
    fn drop(&mut self) {
        self._actor_task.abort();
        if !self.keep_temp_dir && self.temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.temp_dir);
        }
    }
}

impl World {
    pub async fn new() -> Self {
        Self::new_with_default(false).await
    }

    pub async fn new_with_default(default_capture_enabled: bool) -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("netsim_capture_test_{}_{}", std::process::id(), id));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let (runner, client) = capture_actor::new();
        let actor = CaptureActor::new(default_capture_enabled, Some(temp_dir.clone()));
        let actor_task = tokio::spawn(runner.run(actor));

        World { client, temp_dir, keep_temp_dir: false, _actor_task: actor_task }
    }

    pub async fn when_create_capture(
        &self,
        chip_id: u32,
        chip_kind: ChipKind,
        device_name: &str,
        enabled: bool,
    ) -> Result<(), ClientError> {
        let enabled_flag = Arc::new(AtomicBool::new(enabled));
        let create_params =
            CaptureCreate { chip_kind, device_name: device_name.to_string(), enabled_flag };
        CaptureSender::create_capture(&self.client, ChipId(chip_id), create_params).await
    }

    pub async fn when_update_capture(
        &self,
        chip_id: u32,
        enabled: bool,
    ) -> Result<CaptureInfo, ClientError> {
        self.client.update_capture(ChipId(chip_id), enabled).await
    }

    pub async fn when_delete_capture(&self, chip_id: u32) -> Result<(), ClientError> {
        self.client.delete_capture(ChipId(chip_id)).await
    }

    pub async fn when_shutdown(&self) -> Result<(), FrameworkError<CaptureError>> {
        self.client.shutdown().await
    }

    pub async fn then_capture_is_none(&self, chip_id: u32) {
        self.poll_until(|| async {
            let info = self.client.get_capture(ChipId(chip_id)).await.unwrap();
            if info.is_none() { Some(()) } else { None }
        })
        .await;
    }

    pub async fn then_capture_is_enabled(&self, chip_id: u32, expected_enabled: bool) {
        self.poll_until(|| async {
            let info = self.client.get_capture(ChipId(chip_id)).await.unwrap();
            if let Some(info) = info
                && info.enabled == expected_enabled
            {
                return Some(());
            }
            None
        })
        .await;
    }

    pub async fn then_capture_stats_are(
        &self,
        chip_id: u32,
        expected_records: u64,
        expected_bytes: u64,
    ) {
        self.poll_until(|| async {
            let info = self.client.get_capture(ChipId(chip_id)).await.unwrap();
            if let Some(info) = info
                && info.records_written == expected_records
                && info.bytes_written == expected_bytes
            {
                return Some(());
            }
            None
        })
        .await;
    }

    pub async fn when_packet_is_sent(&self, chip_id: u32, packet: &[u8]) {
        let tx =
            self.client.packet_sender(ChipId(chip_id)).await.expect("Failed to get packet sender");
        tx.send((SystemTime::now(), Direction::Sent, Bytes::copy_from_slice(packet)))
            .expect("Failed to send packet");
    }

    pub async fn when_dummy_packet_is_sent(&self, chip_id: u32) {
        self.when_packet_is_sent(chip_id, &[0, 1, 2, 3]).await;
    }

    pub async fn then_capture_file_exists(&self) {
        self.poll_until(|| async {
            let entries: Vec<_> =
                std::fs::read_dir(&self.temp_dir).unwrap().map(|r| r.unwrap().path()).collect();
            if entries.len() == 1 { Some(()) } else { None }
        })
        .await;
    }

    pub async fn then_capture_file_is_not_empty(&self) {
        self.poll_until(|| async {
            let entries: Vec<_> =
                std::fs::read_dir(&self.temp_dir).unwrap().map(|r| r.unwrap().path()).collect();
            if entries.len() == 1 {
                let metadata = std::fs::metadata(&entries[0]).unwrap();
                if metadata.len() > 0 {
                    return Some(());
                }
            }
            None
        })
        .await;
    }

    async fn poll_until<F, Fut, T>(&self, mut f: F) -> T
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Option<T>>,
    {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(1);
        while start.elapsed() < timeout {
            if let Some(res) = f().await {
                return res;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("Timeout waiting for condition");
    }
}
