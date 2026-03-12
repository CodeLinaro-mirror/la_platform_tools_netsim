use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

use actor_framework::{ActorService, DynContext};
use capture_api::{CaptureAction, CaptureActionResult, CaptureCreate, CaptureInfo};
use netsim_model::chip::{ChipId, ChipKind};
use serde::{Deserialize, Serialize};

use crate::{
    bt_pcap::BluetoothH4Writer, capture_actor::CaptureActor, error::CaptureError,
    uwb_pcap::UwbPcapWriter, writer::CaptureWriter,
};

/// Entity representing a packet capture for a specific chip.
///
/// This entity manages the state of a single packet capture session,
/// including whether it is enabled and the associated metadata.
/// It does not hold the actual writer, which is stored in the context
/// to allow shared access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InternalCaptureInfo {
    pub info: CaptureInfo,
    /// Flag to be updated when capture status changes.
    /// This is used by the stream wrappers to check if capture is enabled
    /// without locking the actor.
    #[serde(skip)]
    pub enabled_flag: Arc<AtomicBool>,
    /// Used to suppress logs for calls to [CaptureWriter::write_packet] that
    /// fail.
    #[serde(skip)]
    has_warned_on_write: bool,
}

impl InternalCaptureInfo {
    pub fn from_create_params(id: ChipId, params: CaptureCreate) -> Result<Self, CaptureError> {
        Ok(Self {
            info: CaptureInfo {
                chip_id: id,
                chip_kind: params.chip_kind,
                device_name: params.device_name,
                enabled: params.enabled_flag.load(Ordering::SeqCst),
                records_written: 0,
                bytes_written: 0,
            },
            enabled_flag: params.enabled_flag,
            has_warned_on_write: false,
        })
    }
}

impl CaptureActor {
    pub(crate) async fn create_entity(
        &mut self,
        entity: &mut InternalCaptureInfo,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), CaptureError> {
        // If enabled by default (either via params or context), set up the writer
        if entity.info.enabled || self.default_capture_enabled {
            entity.info.enabled = true;
            entity.enabled_flag.store(true, Ordering::SeqCst);
            self.update_entity(entity, true, ctx).await?;
        }
        Ok(())
    }

    pub(crate) async fn update_entity(
        &mut self,
        entity: &mut InternalCaptureInfo,
        enabled: bool,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), CaptureError> {
        if entity.info.enabled != enabled {
            entity.info.enabled = enabled;
            entity.enabled_flag.store(enabled, Ordering::SeqCst);
        }

        if entity.info.enabled {
            // If enabling, create a writer if one does not exist.
            if !self.writers.contains_key(&entity.info.chip_id) {
                let writer = self.create_writer(entity).await?;
                self.writers.insert(entity.info.chip_id, writer);
            }
        } else {
            // Disable capture: remove the writer to close the file
            if let Some(mut writer) = self.writers.remove(&entity.info.chip_id) {
                if let Err(err) = writer.flush().await {
                    log::warn!("Failed to flush writer for chip {}: {err}", entity.info.chip_id);
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn delete_entity(
        &mut self,
        entity: &InternalCaptureInfo,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), CaptureError> {
        // Clean up resources when the entity is deleted.
        if let Some(mut writer) = self.writers.remove(&entity.info.chip_id) {
            if let Err(err) = writer.flush().await {
                log::warn!("Failed to flush writer for chip {}: {err}", entity.info.chip_id);
            }
        }
        Ok(())
    }

    async fn create_writer(
        &self,
        entity: &InternalCaptureInfo,
    ) -> Result<Box<dyn CaptureWriter>, CaptureError> {
        let _timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!(
            "netsim-{:?}-{:}-{:?}.pcap",
            entity.info.chip_id, entity.info.device_name, entity.info.chip_kind
        );
        let filepath = if let Some(dir) = self.capture_dir.as_ref() {
            dir.join(&filename)
        } else {
            let mut path = common::system::netsimd_temp_dir();
            path.push("pcaps");
            if let Err(e) = std::fs::create_dir_all(&path) {
                log::warn!("Failed to create default pcap directory {}: {}", path.display(), e);
            }
            path.join(&filename)
        };
        log::info!("Creating capture file: {}", filepath.display());
        let writer: Box<dyn CaptureWriter> = match entity.info.chip_kind {
            ChipKind::BLUETOOTH => BluetoothH4Writer::new(&filepath).await?,
            ChipKind::UWB => UwbPcapWriter::new(&filepath).await?,
            ChipKind::WIFI | ChipKind::AP => {
                crate::wifi_pcap::WifiPcapWriter::new(&filepath).await?
            }
            // Fallback
            ChipKind::UNSPECIFIED | ChipKind::NFC | ChipKind::CELLULAR => {
                BluetoothH4Writer::new(&filepath).await?
            }
        };
        Ok(writer)
    }

    pub(crate) fn list_entities(&self) -> Vec<CaptureInfo> {
        self.entities
            .values()
            .map(|e| {
                let (records_written, bytes_written) =
                    self.writers.get(&e.info.chip_id).map(|w| w.get_stats()).unwrap_or((0, 0));
                let mut info = e.info.clone();
                info.records_written = records_written;
                info.bytes_written = bytes_written;
                info
            })
            .collect()
    }
    async fn handle_entity_action(
        &mut self,
        entity: &mut InternalCaptureInfo,
        action: CaptureAction,
        ctx: &mut DynContext<Self>,
    ) -> Result<CaptureActionResult, CaptureError> {
        match action {
            CaptureAction::CapturePacket { chip_id, direction, ref bytes } => {
                if entity.info.enabled && entity.info.chip_id == chip_id {
                    let writer = self.writers.get_mut(&chip_id);
                    if let Some(writer) = writer {
                        if let Err(err) =
                            writer.write_packet(SystemTime::now(), direction, bytes).await
                        {
                            if !entity.has_warned_on_write {
                                entity.has_warned_on_write = true;
                                log::error!("Packet capture write failed for chip {chip_id}: {err}. Further errors for this chip will be suppressed.");
                            }
                        }
                    }
                }
                Ok(CaptureActionResult::Success)
            }

            CaptureAction::Patch { chip_id, enabled } => {
                if entity.info.chip_id == chip_id {
                    self.update_entity(entity, enabled, ctx).await?;
                    let (records_written, bytes_written) =
                        self.writers.get(&chip_id).map(|w| w.get_stats()).unwrap_or((0, 0));
                    let mut info = entity.info.clone();
                    info.records_written = records_written;
                    info.bytes_written = bytes_written;
                    Ok(CaptureActionResult::Updated(info))
                } else {
                    Ok(CaptureActionResult::Success)
                }
            }
            CaptureAction::Delete { chip_id } => {
                if entity.info.chip_id == chip_id {
                    self.delete_entity(entity, ctx).await?;
                }
                Ok(CaptureActionResult::Success)
            }
            CaptureAction::SetCaptureDirectory { ref path } => {
                self.capture_dir = Some(path.clone());
                Ok(CaptureActionResult::Success)
            }
            CaptureAction::Create { .. } => {
                // Handled by create
                Ok(CaptureActionResult::Success)
            }
        }
    }
}

impl ActorService for CaptureActor {
    type Id = ChipId;
    type Create = CaptureCreate;
    type Update = bool; // Enabled status
    type Action = CaptureAction;
    type ActionResult = CaptureActionResult;
    type Error = CaptureError;
    type Entity = CaptureInfo;
    type TypedStream = ();

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let id = params.chip_id;
        let mut entity = InternalCaptureInfo::from_create_params(id, params)?;
        // Initialize the entity logic (e.g. set up writers based on flags)
        self.create_entity(&mut entity, _ctx).await?;

        // 3. Store the entity
        self.entities.insert(id, entity);
        Ok(id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.entities.get(&id).map(|entity| {
            let (records_written, bytes_written) =
                self.writers.get(&id).map(|w| w.get_stats()).unwrap_or((0, 0));
            let mut info = entity.info.clone();
            info.records_written = records_written;
            info.bytes_written = bytes_written;
            info
        }))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        if let Some(mut entity) = self.entities.remove(&id) {
            let _ = self.update_entity(&mut entity, update, _ctx).await?;
            let (records_written, bytes_written) =
                self.writers.get(&id).map(|w| w.get_stats()).unwrap_or((0, 0));
            let mut info = entity.info.clone();
            info.records_written = records_written;
            info.bytes_written = bytes_written;
            self.entities.insert(id, entity);
            Ok(info)
        } else {
            Err(CaptureError::ChipNotFound(id))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        if let Some(entity) = self.entities.remove(&id) {
            self.delete_entity(&entity, _ctx).await
            // Don't re-insert
        } else {
            Err(CaptureError::ChipNotFound(id))
        }
    }

    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        let Some(id) = id else {
            // Global action
            match action {
                CaptureAction::SetCaptureDirectory { path } => {
                    self.capture_dir = Some(path);
                    return Ok(CaptureActionResult::Success);
                }
                _ => {
                    // Other actions require Entity, so they are invalid here (or no-op).
                    return Ok(CaptureActionResult::Success);
                }
            }
        };

        let Some(mut entity) = self.entities.remove(&id) else {
            return Ok(CaptureActionResult::Success);
        };

        let result = self.handle_entity_action(&mut entity, action, ctx).await;

        self.entities.insert(id, entity);
        result
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.list_entities())
    }
}
