use actor_framework::ResourceClient;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use capture_actor::CaptureActor;
use capture_api::{CaptureAction, CaptureActionResult, CaptureCreate, CaptureInfo};
use netsim_model::chip::{ChipId, ChipKind};
use netsim_model::device_error::DeviceError;
use packet_stream::transport::traits::{PacketSink, PacketStream};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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

    /// Creates a new capture for a chip.
    ///
    /// # Arguments
    /// * `chip_id` - The ID of the chip to capture packets from.
    /// * `chip_kind` - The kind of chip (e.g., Bluetooth, Wifi).
    /// * `device_name` - The name of the device associated with the chip.
    pub async fn create_capture(
        &self,
        chip_id: ChipId,
        chip_kind: ChipKind,
        device_name: String,
        default_enabled: bool,
    ) -> Result<(), DeviceError> {
        let create = CaptureCreate {
            chip_id,
            chip_kind,
            device_name,
            default_enabled,
            enabled_flag: Arc::new(AtomicBool::new(default_enabled)),
        };
        self.inner.create(create).await.map_err(|e| DeviceError::Internal(e.to_string()))?;
        Ok(())
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
        if let CaptureActionResult::Success = result {
            Ok(())
        } else {
            Err(anyhow!("Unexpected result from Patch"))
        }
    }

    /// Sets the default capture enabled state for new devices.
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable packet capture by default.
    pub async fn set_default_capture(&self, enabled: bool) -> Result<()> {
        let result = self
            .inner
            .perform_action(None, CaptureAction::SetDefaultCapture { enabled })
            .await
            .map_err(|e| anyhow!(e))?;
        match result {
            CaptureActionResult::Success => Ok(()),
            _ => Err(anyhow!("Unexpected result from SetDefaultCapture")),
        }
    }

    /// Captures a single packet.
    ///
    /// # Arguments
    /// * `chip_id` - The ID of the chip.
    /// * `direction` - The direction of the packet.
    /// * `bytes` - The packet data.
    pub fn capture_packet(&self, chip_id: ChipId, direction: capture_api::Direction, bytes: Bytes) {
        let inner = self.inner.clone();
        // TODO(b/312345678): Consider using a sync channel and a single background task
        // instead of spawning a new task for every packet if performance becomes an issue.
        tokio::spawn(async move {
            let _ = inner
                .perform_action(
                    Some(chip_id),
                    CaptureAction::CapturePacket { chip_id, direction, bytes },
                )
                .await;
        });
    }

    /// Wraps a PacketStream and PacketSink for automatic packet capture.
    ///
    /// # Arguments
    /// * `chip_id` - The ID of the chip.
    /// * `stream` - The PacketStream to wrap (for received packets).
    /// * `sink` - The PacketSink to wrap (for sent packets).
    /// * `enabled` - An AtomicBool flag to control capture.
    pub fn wrap_stream_sink(
        &self,
        chip_id: ChipId,
        stream: PacketStream,
        sink: PacketSink,
        enabled: Arc<AtomicBool>,
    ) -> (capture_api::io::CapturedStream, capture_api::io::CapturedSink) {
        let client = self.clone();
        let capture_callback = Box::new(move |chip_id, direction, bytes| {
            client.capture_packet(chip_id, direction, bytes);
        });
        let _capture_callback_clone = capture_callback.clone(); // Need to clone for Sink, but Box<dyn Fn> isn't Clone.
                                                                // Wait, I need two callbacks.
        let client_clone = self.clone();
        let capture_callback_sink = Box::new(move |chip_id, direction, bytes| {
            client_clone.capture_packet(chip_id, direction, bytes);
        });

        use futures::{SinkExt, StreamExt};
        let stream_unpinned: netsim_model::chip::PacketStream =
            Box::new(stream.filter_map(|item| {
                futures::future::ready(match item {
                    Ok(bytes) => Some(bytes),
                    Err(_) => None, // Handle error or log it
                })
            }));
        let sink_mapped: netsim_model::chip::PacketSink =
            Box::pin(sink.sink_map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

        (
            capture_api::io::CapturedStream::new(
                stream_unpinned,
                capture_callback,
                chip_id,
                enabled.clone(),
            ),
            capture_api::io::CapturedSink::new(
                sink_mapped,
                capture_callback_sink,
                chip_id,
                enabled,
            ),
        )
    }
}
#[async_trait]
impl capture_api::CaptureSender for CaptureClient {
    async fn create_capture(&self, create: capture_api::CaptureCreate) -> anyhow::Result<()> {
        self.create_capture(
            create.chip_id,
            create.chip_kind,
            create.device_name,
            create.default_enabled,
        )
        .await
        .map_err(|e| anyhow::anyhow!(e))
    }
    fn capture_packet(&self, chip_id: ChipId, direction: capture_api::Direction, packet: Bytes) {
        self.capture_packet(chip_id, direction, packet);
    }
}
