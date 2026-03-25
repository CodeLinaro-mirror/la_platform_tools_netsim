// Copyright 2026 The Android Open Source Project

//! Capture Client

use std::ops::Deref;

use actor_framework::ResourceClient;
use anyhow::Result;
use bytes::Bytes;
use capture_api::{
    CaptureAction, CaptureActionResult, CaptureCreate, CaptureInfo, CaptureSender, Direction,
};
use netsim_model::ChipId;

use crate::CaptureActor;

/// A client for interacting with the Capture Actor.
///
/// This client provides a type-safe API for managing packet captures.
#[derive(Clone, Debug)]
pub struct CaptureClient {
    pub(crate) inner: ResourceClient<CaptureActor>,
}

#[async_trait::async_trait]
impl CaptureSender for CaptureClient {
    async fn create_capture(&self, chip_id: ChipId, create: CaptureCreate) -> anyhow::Result<()> {
        self.inner.create_with_id(chip_id, create).await.map(|_| ()).map_err(|e| anyhow::anyhow!(e))
    }

    async fn packet_sender(
        &self,
        chip_id: ChipId,
    ) -> anyhow::Result<tokio::sync::mpsc::UnboundedSender<(std::time::SystemTime, Direction, Bytes)>>
    {
        let result = self
            .inner
            .perform_action(Some(chip_id), CaptureAction::GetPacketSender)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        match result {
            CaptureActionResult::PacketSender(sender) => Ok(sender),
            _ => Err(anyhow::anyhow!("Unexpected result from GetPacketSender")),
        }
    }
}

impl CaptureClient {
    pub fn new(inner: ResourceClient<CaptureActor>) -> Self {
        Self { inner }
    }

    /// Creates a new capture session for a chip.
    pub async fn create_capture(&self, params: CaptureCreate) -> Result<ChipId> {
        self.inner.create(params).await.map_err(|e| anyhow::anyhow!(e))
    }

    /// Updates the capture state (enable/disable) for a chip.
    pub async fn update_capture(&self, chip_id: ChipId, enabled: bool) -> Result<CaptureInfo> {
        let result = self.inner.update(chip_id, enabled).await.map_err(|e| anyhow::anyhow!(e))?;
        Ok(result)
    }

    /// Gets the capture info for a chip.
    pub async fn get_capture(&self, chip_id: ChipId) -> Result<Option<CaptureInfo>> {
        self.inner.get(chip_id).await.map_err(|e| anyhow::anyhow!(e))
    }

    /// Deletes a capture session.
    pub async fn delete_capture(&self, chip_id: ChipId) -> Result<()> {
        self.inner.delete(chip_id).await.map_err(|e| anyhow::anyhow!(e))
    }

    /// Lists all active captures.
    pub async fn list_captures(&self) -> Result<Vec<CaptureInfo>> {
        self.inner.list().await.map_err(|e| anyhow::anyhow!(e))
    }

    /// Patches a capture (alias for update_capture to match old client).
    pub async fn patch_capture(&self, chip_id: ChipId, enabled: bool) -> Result<()> {
        self.update_capture(chip_id, enabled).await.map(|_| ())
    }
}

impl Deref for CaptureClient {
    type Target = ResourceClient<CaptureActor>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
