use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use actor_framework::{ActorService, Context, DynContext};
use async_trait::async_trait;
use capture_api::CaptureSender;
use device_api::{
    api::{DeviceCreate, DeviceUpdate},
    DeviceAction, DeviceActionResult, DeviceAddChip, DeviceId,
};
use futures::future::join_all;
use link_api::LinkClient;
use netsim_model::chip::{
    Chip, ChipClient, ChipConfig, ChipCreate, ChipId, ChipKind, ChipUpdate, ChipVariant,
    PacketSink, PacketStream,
};
use serde::{Deserialize, Serialize};

use crate::{
    device_actor::DeviceActor,
    error::DeviceError,
    utils::{create_capture_and_wrap_streams, StreamStats},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InternalDevice {
    pub device: device_api::Device,
    pub create_params: Option<DeviceCreate>,
    pub guid: Option<String>,
    #[serde(skip)]
    pub chip_stats: HashMap<ChipId, Arc<StreamStats>>,
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
            chip_stats: HashMap::new(),
        })
    }
}

impl DeviceActor {
    fn update_idle_state(&mut self, ctx: &mut DynContext<Self>) {
        let has_active_devices = self.devices.values().any(|d| !d.device.builtin);

        if has_active_devices {
            self.has_seen_device = true;
            if let Some(key) = self.startup_timer.take() {
                ctx.cancel_timer(key);
            }
            if let Some(key) = self.idle_timer.take() {
                ctx.cancel_timer(key);
            }
        } else if self.has_seen_device && self.idle_timer.is_none() {
            if let Some(timeout) = self.idle_timeout {
                log::info!("DeviceActor: Scheduling idle shutdown in {:?}", timeout);
                let key = ctx.run_later(timeout, Box::new(Self::on_idle_timeout));
                self.idle_timer = Some(key);
            }
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

        // 2. Wrap streams for stats and optional packet capture.
        let (packet_stream, packet_sink, stream_stats) = create_capture_and_wrap_streams(
            capture_client.clone(),
            chip_id,
            chip_kind,
            entity.device.name.clone(),
            packet_stream,
            packet_sink,
        )
        .await;

        if let Some(stats) = stream_stats {
            entity.chip_stats.insert(chip_id, stats);
        }

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
        ctx: &mut DynContext<Self>,
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
        self.stats.update_device_count(self.devices.len(), true);
        self.update_idle_state(ctx);
        Ok(id)
    }

    async fn perform_add_chip_by_guid(
        &mut self,
        params: DeviceAddChip,
        ctx: &mut DynContext<Self>,
    ) -> Result<DeviceActionResult, DeviceError> {
        log::info!("DeviceActor: AddChipByGuid for device {}", params.device_guid);

        if let Some(id) = self.guid_to_id.get(&params.device_guid) {
            // Add Chip to Existing Device
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
            // Create New Device
            let chip_create_params = params.chip_config.clone().into();
            let create_params =
                DeviceCreate { device_config: params.device_config, chip: chip_create_params };

            let id = self
                .perform_create_device(
                    None,
                    create_params,
                    params.packet_stream,
                    params.packet_sink,
                    ctx,
                )
                .await?;

            // Update GUID mapping
            if let Some(entity) = self.devices.get_mut(&id) {
                entity.guid = Some(params.device_guid.clone());
            }
            self.guid_to_id.insert(params.device_guid, id);

            // Get the chip id (it's the first one, as we just created the device)
            let chip_id = self
                .devices
                .get(&id)
                .ok_or_else(|| DeviceError::DeviceNotFound(id.to_string()))?
                .device
                .chips
                .first()
                .ok_or_else(|| DeviceError::DeviceNotFound("Device created without chips".into()))?
                .id;

            Ok(DeviceActionResult::AddChipByGuidSuccess { device_id: id, chip_id: ChipId(chip_id) })
        }
    }

    /// Callback for startup timeout
    pub(crate) fn on_startup_timeout(&mut self, ctx: &mut dyn Context<Self>) {
        let has_active_devices = self.devices.values().any(|d| !d.device.builtin);
        if !self.has_seen_device && !has_active_devices {
            log::info!(
                "DeviceActor: Startup timeout reached (no devices connected), shutting down"
            );
            ctx.shutdown();
        }
        self.startup_timer = None;
    }

    /// Callback for idle timeout
    pub(crate) fn on_idle_timeout(&mut self, ctx: &mut dyn Context<Self>) {
        let has_active_devices = self.devices.values().any(|d| !d.device.builtin);
        if !has_active_devices {
            log::info!("DeviceActor: Idle timeout reached, shutting down");
            ctx.shutdown();
        }
        self.idle_timer = None;
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
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        self.perform_create_device(id, params, None, None, ctx).await
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.devices.get(&id).map(|e| e.device.clone()))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        // Update does not affect device count, so no timeout logic change needed.
        let Some(entity) = self.devices.get_mut(&id) else {
            return Err(DeviceError::DeviceNotFound(id.to_string()));
        };
        // Update local state
        if let Some(name) = &update.name {
            entity.device.name = name.clone();
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
            let Some(chip_client) = self.chip_clients.get(&chip.kind) else {
                continue;
            };

            let mut chip_update = ChipUpdate::default();

            // Propagate Device Position/Orientation if changed
            if update.position.is_some() {
                chip_update.position = update.position.clone();
            }
            if update.orientation.is_some() {
                chip_update.orientation = update.orientation.clone();
            }

            // Start with ID-based matching
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

            // Merge specific update fields
            if let Some(u) = specific_update {
                if u.variant.is_some() {
                    chip_update.variant = u.variant.clone();
                }
            }

            // Send update if meaningful
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
                *chip = chip_client
                    .update(ChipId(chip.id), chip_update)
                    .await
                    .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            }
        }
        Ok(entity.device.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        let Some(mut internal_device) = self.devices.remove(&id) else {
            return Err(DeviceError::DeviceNotFound(id.to_string()));
        };
        self.stats.update_device_count(self.devices.len(), false);

        if let Some(guid) = &internal_device.guid {
            self.guid_to_id.remove(guid);
        }
        for chip in &internal_device.device.chips {
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

        self.update_idle_state(ctx);
        Ok(())
    }

    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        let Some(id) = id else {
            // Global actions
            return match action {
                DeviceAction::Reset => {
                    // TODO: Implement global reset logic
                    Ok(DeviceActionResult::Success)
                }
                DeviceAction::GetRadioStats => {
                    // Aggregate stats from all chips concurrently
                    let futures = self.chip_clients.values().map(|client| client.read_statistics());
                    let results = join_all(futures).await;

                    let mut all_stats = Vec::new();
                    for result in results {
                        match result {
                            Ok(stats) => all_stats.extend(stats.into_vec()),
                            Err(e) => log::warn!(
                                "DeviceActor: Failed to collect radio stats from chip: {}",
                                e
                            ),
                        }
                    }

                    // Optimization: Build a flat map of duplicate-safe stats first (O(N))
                    let transport_stats: HashMap<ChipId, Arc<StreamStats>> = self
                        .devices
                        .values()
                        .flat_map(|d| d.chip_stats.iter())
                        .map(|(k, v)| (*k, v.clone()))
                        .collect();

                    for stats in &mut all_stats {
                        if let Some(stream_stats) = transport_stats.get(&ChipId(stats.id)) {
                            stats.tx_count = stream_stats.rx_packets.load(Ordering::Relaxed);
                            stats.tx_bytes = stream_stats.rx_bytes.load(Ordering::Relaxed);
                            stats.rx_count = stream_stats.tx_packets.load(Ordering::Relaxed);
                            stats.rx_bytes = stream_stats.tx_bytes.load(Ordering::Relaxed);
                            stats.duration_secs = stream_stats.start_time.elapsed().as_secs();
                        }
                    }

                    Ok(DeviceActionResult::Statistics(all_stats))
                }
                DeviceAction::AddChipByGuid { params } => {
                    self.perform_add_chip_by_guid(params, ctx).await
                }
                _ => Err(DeviceError::NotFound("Action requires a device ID".into())),
            };
        };

        let Some(entity) = self.devices.get_mut(&id) else {
            return Err(DeviceError::DeviceNotFound(id.to_string()));
        };

        match action {
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
                    self.handle_delete(device_id, ctx).await?;
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
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.devices.values().map(|e| e.device.clone()).collect())
    }
}
