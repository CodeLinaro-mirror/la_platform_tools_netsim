use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use capture_api::CaptureSender;
use device_api::{
    api::{DeviceCreate, DeviceUpdate},
    DeviceAction, DeviceActionResult, DeviceAddChip, DeviceId,
};
use link_api::LinkClient;
use netsim_model::chip::{
    Chip, ChipClient, ChipConfig, ChipCreate, ChipId, ChipKind, ChipUpdate, ChipVariant,
    PacketSink, PacketStream,
};
use serde::{Deserialize, Serialize};

use crate::{
    device_actor::DeviceActor, error::DeviceError, utils::create_capture_and_wrap_streams,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InternalDevice {
    pub device: device_api::Device,
    pub create_params: Option<DeviceCreate>,
    pub guid: Option<String>,
}

impl InternalDevice {
    fn from_create_params(id: DeviceId, params: DeviceCreate) -> Result<Self, DeviceError> {
        Ok(InternalDevice {
            device: device_api::Device {
                id: id.0,
                name: params.device_config.name.clone(),
                visible: params.device_config.visible,
                position: params.device_config.position.clone(),
                orientation: params.device_config.orientation.clone(),
                builtin: params.device_config.builtin,
                chips: vec![],
            },
            create_params: Some(params),
            guid: None,
        })
    }
}

impl DeviceActor {
    fn update_idle_state(&mut self) {
        let has_active_devices = self.devices.values().any(|d| {
            // Builtin devices (like infra) don't count as "user activity"
            if d.device.builtin {
                return false;
            }
            true
        });

        if has_active_devices {
            self.has_seen_device = true;
            self.last_empty_time = None;
        } else if self.has_seen_device && self.last_empty_time.is_none() {
            self.last_empty_time = Some(std::time::Instant::now());
        }
    }

    /// Adds a chip to a device.
    #[allow(clippy::too_many_arguments)]
    async fn perform_add_chip(
        next_chip_id: &Arc<AtomicU32>,
        chip_clients: &HashMap<ChipKind, Box<dyn ChipClient>>,
        link_client: &Box<dyn LinkClient>,
        capture_client: &Option<Arc<dyn CaptureSender>>,
        entity: &mut InternalDevice,
        chip_config: ChipConfig,
        packet_stream: Option<PacketStream>,
        packet_sink: Option<PacketSink>,
    ) -> Result<ChipId, DeviceError> {
        log::info!("DeviceActor: AddChip {} to device {}", chip_config.name, entity.device.name);

        // 1. Prepare Chip Parameters
        let chip_id = ChipId(next_chip_id.fetch_add(1, Ordering::SeqCst));
        let chip_kind_params = chip_config.chip_kind_params.clone();
        let chip_kind = ChipKind::from(&chip_kind_params);

        // 2. Handle Capture Creation and Stream Wrapping
        // If a capture client is present, wrap the streams to enable packet capture.
        let (packet_stream, packet_sink) = if let Some(capture_client) = capture_client {
            create_capture_and_wrap_streams(
                capture_client.clone(),
                chip_id,
                chip_kind,
                entity.device.name.clone(),
                packet_stream,
                packet_sink,
            )
            .await
        } else {
            (packet_stream, packet_sink)
        };

        // 3. Get Chip Client
        let chip_client = chip_clients.get(&chip_kind).ok_or_else(|| {
            DeviceError::ChipKindNotSupported(format!("No chip client for {:?}", chip_kind))
        })?;

        // 4. Send Create Request to Chip Actor
        let chip_create_params = ChipCreate {
            id: chip_id,
            packet_stream,
            packet_sink,
            config: netsim_model::chip::ChipConfig {
                name: chip_config.name.clone(),
                manufacturer: chip_config.manufacturer.clone(),
                product_name: chip_config.product_name.clone(),
                chip_kind_params,
            },
            device_id: DeviceId(entity.device.id),
        };

        chip_client
            .create(chip_create_params)
            .await
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

        // 5. Update Local Device State
        entity.device.chips.push(Chip {
            id: chip_id.0,
            kind: ChipKind::from(&chip_config.chip_kind_params),
            name: Some(chip_config.name),
            manufacturer: Some(chip_config.manufacturer),
            product_name: Some(chip_config.product_name),
            position: entity.device.position.clone(),
            orientation: entity.device.orientation.clone(),
            device_id: DeviceId(entity.device.id),
            variant: Some(ChipVariant::from(chip_kind)),
            links: vec![],
            enabled: true,
        });

        // 6. Notify Link Actor
        link_client
            .notify_chip_added(chip_id, chip_kind)
            .await
            .expect("Failed to notify LinkActor of chip add");

        Ok(chip_id)
    }

    async fn perform_create_device(
        &mut self,
        id: Option<DeviceId>,
        params: DeviceCreate,
        packet_stream: Option<PacketStream>,
        packet_sink: Option<PacketSink>,
    ) -> Result<DeviceId, DeviceError> {
        log::info!("DeviceActor: Create device {}", params.device_config.name);
        let id = id.unwrap_or_else(|| {
            let id = DeviceId(self.next_device_id);
            self.next_device_id += 1;
            id
        });

        let mut entity = InternalDevice::from_create_params(id, params.clone())?;
        let chip_config: ChipConfig = params.chip.into();
        Self::perform_add_chip(
            &self.next_chip_id,
            &self.chip_clients,
            &self.link_client,
            &self.capture_client,
            &mut entity,
            chip_config,
            packet_stream,
            packet_sink,
        )
        .await?;

        self.devices.insert(id, entity);
        self.update_idle_state();
        Ok(id)
    }

    async fn perform_add_chip_by_guid(
        &mut self,
        params: DeviceAddChip,
    ) -> Result<DeviceActionResult, DeviceError> {
        log::info!("DeviceActor: AddChipByGuid for device {}", params.device_guid);

        // Check if device exists
        if let Some(id) = self.guid_to_id.get(&params.device_guid) {
            // Device Exists: Add Chip
            let id = *id;
            let entity = self
                .devices
                .get_mut(&id)
                .ok_or_else(|| DeviceError::DeviceNotFound(id.to_string()))?;

            let chip_id = Self::perform_add_chip(
                &self.next_chip_id,
                &self.chip_clients,
                &self.link_client,
                &self.capture_client,
                entity,
                params.chip_config,
                params.packet_stream,
                params.packet_sink,
            )
            .await?;

            Ok(DeviceActionResult::AddChipByGuidSuccess { device_id: id, chip_id })
        } else {
            // Device Does Not Exist: Create New Device
            let chip_create_params = params.chip_config.clone().into();
            let create_params =
                DeviceCreate { device_config: params.device_config, chip: chip_create_params };

            let id = self
                .perform_create_device(
                    None,
                    create_params,
                    params.packet_stream,
                    params.packet_sink,
                )
                .await?;

            // Update GUID mapping
            if let Some(entity) = self.devices.get_mut(&id) {
                entity.guid = Some(params.device_guid.clone());
            }
            self.guid_to_id.insert(params.device_guid, id);

            // Get the chip id (it's the first one, as we just created the device)
            let chip_id = self.devices.get(&id).unwrap().device.chips[0].id;

            Ok(DeviceActionResult::AddChipByGuidSuccess { device_id: id, chip_id: ChipId(chip_id) })
        }
    }
}

#[async_trait]
impl ActorService for DeviceActor {
    type Id = DeviceId;
    type Create = DeviceCreate;
    type Update = DeviceUpdate;
    type Action = DeviceAction;
    type ActionResult = DeviceActionResult;
    type Error = DeviceError;
    type Entity = device_api::Device;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        self.perform_create_device(id, params, None, None).await
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.devices.get(&id).map(|e| e.device.clone()))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        // Update does not affect device count, so no timeout logic change needed.
        let Some(entity) = self.devices.get_mut(&id) else {
            return Err(DeviceError::DeviceNotFound(id.to_string()));
        };
        // Update local state
        if let Some(name) = update.name {
            entity.device.name = name;
        }
        if let Some(visible) = update.visible {
            entity.device.visible = visible;
        }
        //TODO: check if chip_id is valid
        if let Some(pos) = update.position.clone() {
            entity.device.position = pos;
        }
        if let Some(orient) = update.orientation.clone() {
            entity.device.orientation = orient;
        }

        // Propagate updates to chips
        for chip in entity.device.chips.iter_mut() {
            if let Some(chip_client) = self.chip_clients.get(&chip.kind) {
                let mut chip_update = ChipUpdate::default();

                // 1. Propagate Device Position/Orientation if changed
                if update.position.is_some() {
                    chip_update.position = Some(entity.device.position.clone());
                }
                if update.orientation.is_some() {
                    chip_update.orientation = Some(entity.device.orientation.clone());
                }

                // 2. Start with ID-based matching
                let mut specific_update = None;
                if let Some(chips) = &update.chips {
                    // Priority 1: Exact ID match
                    specific_update = chips.iter().find(|u| u.id == Some(ChipId(chip.id)));

                    // Priority 2: Variant match (if no ID match found)
                    if specific_update.is_none() {
                        specific_update = chips.iter().find(|u| {
                            u.id.is_none()
                                && u.variant.as_ref().map_or(false, |v| v.kind() == chip.kind)
                        });
                    }
                }

                // 3. Merge specific update fields
                if let Some(u) = specific_update {
                    if u.variant.is_some() {
                        chip_update.variant = u.variant.clone();
                    }
                }

                // TODO: Propagate SSID update to AP chips if the device name changes

                // 4. Send update if meaningful
                if chip_update.position.is_some()
                    || chip_update.orientation.is_some()
                    || chip_update.variant.is_some()
                {
                    log::info!(
                        "DeviceActor: Updating chip {} (kind {:?}) with {:?}",
                        chip.id,
                        chip.kind,
                        chip_update
                    );
                    let updated_chip = chip_client
                        .update(ChipId(chip.id), chip_update)
                        .await
                        .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
                    *chip = updated_chip;
                }
            }
        }
        Ok(entity.device.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        if let Some(entity) = self.devices.remove(&id) {
            if let Some(guid) = &entity.guid {
                self.guid_to_id.remove(guid);
            }
            for chip in &entity.device.chips {
                if let Some(chip_client) = self.chip_clients.get(&chip.kind) {
                    chip_client
                        .delete(ChipId(chip.id))
                        .await
                        .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
                    // Send delete request to Link Actor
                    self.link_client
                        .notify_chip_removed(ChipId(chip.id))
                        .await
                        .expect("Failed to notify LinkActor of chip remove");
                }
            }
            if self.devices.is_empty() {
                self.last_empty_time = Some(std::time::Instant::now());
            } else {
                self.update_idle_state();
            }
            Ok(())
        } else {
            Err(DeviceError::DeviceNotFound(id.to_string()))
        }
    }

    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        let Some(id) = id else {
            // Global actions
            return match action {
                DeviceAction::Reset => {
                    // TODO: Implement global reset logic
                    Ok(DeviceActionResult::Success)
                }
                DeviceAction::AddChipByGuid { params } => {
                    self.perform_add_chip_by_guid(params).await
                }
                _ => Err(DeviceError::NotFound("Action requires a device ID".into())),
            };
        };

        let Some(entity) = self.devices.get_mut(&id) else {
            return Err(DeviceError::DeviceNotFound(id.to_string()));
        };

        let result = match action {
            DeviceAction::Reset => {
                // TODO: Implement device reset logic if needed
                Ok(DeviceActionResult::Success)
            }
            DeviceAction::NotifyChipRemoved(device_id, chip_id) => {
                let should_delete = {
                    let entity = self
                        .devices
                        .get_mut(&device_id)
                        .ok_or_else(|| DeviceError::DeviceNotFound(device_id.to_string()))?;
                    entity.device.chips.retain(|c| c.id != chip_id.0);
                    entity.device.chips.is_empty()
                };

                self.link_client
                    .notify_chip_removed(chip_id)
                    .await
                    .expect("Failed to notify LinkActor of chip remove");

                if should_delete {
                    log::info!("DeviceActor: Device {} is empty, auto-deleting", device_id);
                    self.handle_delete(device_id, _ctx).await?;
                } else {
                    log::info!("DeviceActor: Device {} is NOT empty after chip removal", device_id);
                }

                Ok(DeviceActionResult::Success)
            }
            DeviceAction::AddChip { chip_config, packet_stream, packet_sink } => {
                // Convert API ChipConfig to Model ChipConfig
                let config: ChipConfig = chip_config.into();
                let chip_id_res = Self::perform_add_chip(
                    &self.next_chip_id,
                    &self.chip_clients,
                    &self.link_client,
                    &self.capture_client,
                    entity,
                    config,
                    packet_stream,
                    packet_sink,
                )
                .await;
                match chip_id_res {
                    Ok(chip_id) => Ok(DeviceActionResult::ChipId(chip_id)),
                    Err(e) => Err(e),
                }
            }
            _ => Err(DeviceError::NotFound("Action requires a device ID".into())),
        };
        self.update_idle_state();
        result
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.devices.values().map(|e| e.device.clone()).collect())
    }
}
