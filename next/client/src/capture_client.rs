use actor_framework::ResourceClient;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use capture_actor::CaptureActor;
use capture_api::{CaptureAction, CaptureActionResult, CaptureInfo};
use netsim_model::{chip::ChipId, device_error::DeviceError};

/// Client for interacting with the CaptureActor.
///
/// This client provides a type-safe interface for managing packet captures,
/// including creating, deleting, listing, and patching captures.
#[derive(Clone, Debug)]
pub struct CaptureClient {
    inner: ResourceClient<CaptureActor>,
}

impl CaptureClient {
    /// Creates a new CaptureClient.
    pub fn new(inner: ResourceClient<CaptureActor>) -> Self {
        Self { inner }
    }

    /// Deletes an existing capture.
    ///
    /// # Arguments
    /// * `chip_id` - The ID of the chip whose capture should be deleted.
    pub async fn delete_capture(&self, chip_id: ChipId) -> Result<()> {
        self.inner.delete(chip_id).await.map_err(|e| anyhow!(e))?;
        Ok(())
    }

    /// Lists all active captures.
    ///
    /// Returns a list of `CaptureInfo` for all captures managed by the actor.
    pub async fn list_captures(&self) -> Result<Vec<CaptureInfo>> {
        self.inner.list().await.map_err(|e| anyhow!(e))
    }

    /// Gets information about a specific capture.
    ///
    /// # Arguments
    /// * `chip_id` - The ID of the chip to get capture info for.
    pub async fn get_capture(&self, chip_id: ChipId) -> Result<Option<CaptureInfo>> {
        self.inner.get(chip_id).await.map_err(|e| anyhow!(e))
    }

    /// Patches a capture, e.g., enabling or disabling it.
    ///
    /// # Arguments
    /// * `chip_id` - The ID of the chip to patch.
    /// * `enabled` - Whether to enable or disable packet capture.
    pub async fn patch_capture(&self, chip_id: ChipId, enabled: bool) -> Result<()> {
        let result = self
            .inner
            .perform_action(Some(chip_id), CaptureAction::Patch { chip_id, enabled })
            .await
            .map_err(|e| anyhow!(e))?;
        match result {
            CaptureActionResult::Success | CaptureActionResult::Updated(_) => Ok(()),
            CaptureActionResult::Error(err) => {
                Err(anyhow!("Unexpected result from Patch: {err:?}"))
            }
        }
    }
}

#[async_trait]
impl capture_api::CaptureSender for CaptureClient {
    async fn create_capture(&self, create: capture_api::CaptureCreate) -> anyhow::Result<()> {
        self.inner.create(create).await.map_err(|e| DeviceError::Internal(e.to_string()))?;
        Ok(())
    }

    fn capture_packet(&self, chip_id: ChipId, direction: capture_api::Direction, bytes: Bytes) {
        let inner = self.inner.clone();
        // TODO(b/487016047): Consider using a sync channel and a single background task
        // instead of spawning a new task for every packet if performance becomes an
        // issue.
        tokio::spawn(async move {
            let _ = inner
                .perform_action(
                    Some(chip_id),
                    CaptureAction::CapturePacket { chip_id, direction, bytes },
                )
                .await;
        });
    }
}
