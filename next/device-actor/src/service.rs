// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use actor_framework::{ActorService, Context, DynContext};
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
use netsim_proto::protobuf::Message;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{
    device_actor::DeviceActor,
    error::DeviceError,
    utils::{create_capture_and_wrap_streams, to_proto_stats, StreamStats},
};

const CHIP_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InternalDevice {
    pub device: device_api::Device,
    pub create_params: Option<DeviceCreate>,
    pub guid: Option<String>,
    #[serde(skip)]
    pub chip_stats: HashMap<ChipId, Arc<StreamStats>>,
    #[serde(skip)]
    pub last_known_stats: HashMap<ChipId, Vec<netsim_model::stats::NetsimRadioStats>>,
}

impl InternalDevice {
    fn from_create_params(id: DeviceId, params: DeviceCreate) -> Result<Self, DeviceError> {
        Ok(InternalDevice {
            device: device_api::Device {
                id: id.0,
                name: params.device_config.name.clone(),
                visible: params.device_config.visible,
                pose: params.device_config.pose,
                builtin: params.device_config.builtin,
                chips: vec![],
                device_info: params.device_config.device_info.clone(),
            },
            create_params: Some(params),
            guid: None,
            chip_stats: HashMap::new(),
            last_known_stats: HashMap::new(),
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
                info!("DeviceActor: Scheduling idle shutdown in {:?}", timeout);
                let key = ctx.run_later(timeout, Box::new(Self::on_idle_timeout));
                self.idle_timer = Some(key);
            }
        }
    }

    /// Collects radio stats from local atomic counters (synchronous).
    /// This is preferred for persistence as it doesn't require IPC to chip
    /// actors.
    pub(crate) async fn save_stats_async(&mut self) {
        let active_model_stats = self.collect_radio_stats_async().await;
        // Convert to Proto for persistence
        let active_proto_stats = active_model_stats.into_iter().map(to_proto_stats).collect();

        // Collect WiFi specific stats (Global)
        let wifi_stats = self.collect_wifi_stats_async().await;

        let combined_stats = self.stats.get_combined_stats(active_proto_stats, wifi_stats);
        let path = self.stats.stats_path.clone();

        let previous_task = self.stats_write_task.take();

        // Spawn a background async task to prevent the actor from blocking on disk I/O.
        // It awaits the previous write task to ensure sequential writes without race
        // conditions.
        let task = tokio::spawn(async move {
            if let Some(t) = previous_task {
                if let Err(e) = t.await {
                    warn!("DeviceActor: Previous stats write failed: {}", e);
                }
            }

            // Spawn blocking write for disk I/O
            let res = tokio::task::spawn_blocking(move || {
                if let Err(e) = crate::stats::write_combined_stats(combined_stats, path) {
                    error!("Failed to write stats: {}", e);
                }
            })
            .await;

            if let Err(e) = res {
                error!("DeviceActor: Stats write task panicked: {}", e);
            }
        });

        self.stats_write_task = Some(task);
    }

    async fn collect_radio_stats_async(&mut self) -> Vec<netsim_model::stats::NetsimRadioStats> {
        let mut stats_futures = Vec::new();
        let mut state_futures = Vec::new();

        for (_kind, client) in &self.chip_clients {
            // Stats requests
            stats_futures.push(async move {
                tokio::time::timeout(CHIP_READ_TIMEOUT, client.read_statistics())
                    .await
                    .ok()
                    .and_then(|res| res.ok())
            });
        }

        // Identify BT chips for state fetching
        let mut bt_chips = Vec::new();
        for device in self.devices.values() {
            for chip in &device.device.chips {
                if chip.kind == netsim_model::ChipKind::BLUETOOTH {
                    bt_chips.push((chip.kind, chip.id));
                }
            }
        }

        for (kind, id) in bt_chips {
            if let Some(client) = self.chip_clients.get(&kind) {
                let id = ChipId(id);
                state_futures.push(async move {
                    let res = tokio::time::timeout(CHIP_READ_TIMEOUT, client.read(id))
                        .await
                        .ok()
                        .and_then(|res| res.ok());
                    (id, res)
                });
            }
        }

        let stats_results = futures::future::join_all(stats_futures).await;
        let state_results = futures::future::join_all(state_futures).await;

        let mut chip_stats_map: HashMap<ChipId, Vec<netsim_model::stats::NetsimRadioStats>> =
            HashMap::new();
        for stats in stats_results.into_iter().flatten() {
            for stat in stats {
                chip_stats_map.entry(ChipId(stat.id)).or_default().push(stat);
            }
        }

        let mut chip_state_map: HashMap<ChipId, Chip> = HashMap::new();
        for (id, res) in state_results {
            if let Some(chip) = res {
                chip_state_map.insert(id, chip);
            }
        }

        let mut stats_list = Vec::new();
        for device_entry in self.devices.values_mut() {
            for chip in &device_entry.device.chips {
                let stream_stats = device_entry.chip_stats.get(&ChipId(chip.id));
                let duration_secs =
                    stream_stats.map(|s| s.start_time.elapsed().as_secs()).unwrap_or(0);

                if let Some(client_stats) =
                    chip_stats_map.get(&ChipId(chip.id)).filter(|v| !v.is_empty())
                {
                    let mut chip_stats: Vec<netsim_model::stats::NetsimRadioStats> = client_stats
                        .iter()
                        .map(|stat| {
                            let mut radio_stats = stat.clone();
                            radio_stats.id = device_entry.device.id;
                            radio_stats.duration_secs = duration_secs;
                            radio_stats
                        })
                        .collect();

                    device_entry.last_known_stats.insert(ChipId(chip.id), chip_stats.clone());
                    if let Some(stream_stats) = stream_stats {
                        crate::utils::distribute_stream_stats(&mut chip_stats, stream_stats);
                    }
                    stats_list.extend(chip_stats);
                    continue;
                }

                if let Some(cached_stats) = device_entry.last_known_stats.get(&ChipId(chip.id)) {
                    let mut chip_stats = cached_stats.clone();
                    for stat in &mut chip_stats {
                        stat.duration_secs = duration_secs;
                    }
                    if let Some(stream_stats) = stream_stats {
                        crate::utils::distribute_stream_stats(&mut chip_stats, stream_stats);
                    }
                    stats_list.extend(chip_stats);
                    continue;
                }

                if let Some(ss) = stream_stats {
                    // Resolve Kind Synchronously using pre-fetched state
                    let radio_kind = if chip.kind == netsim_model::ChipKind::BLUETOOTH {
                        let chip_state = chip_state_map.get(&ChipId(chip.id)).unwrap_or(chip);
                        Self::resolve_bluetooth_kind(chip, Some(chip_state))
                    } else {
                        Some(netsim_model::chip::chip_kind_to_radio_kind(chip.kind))
                    };

                    if let Some(kind) = radio_kind {
                        stats_list.push(crate::utils::stream_to_model_stats(
                            device_entry.device.id,
                            kind,
                            duration_secs,
                            ss,
                        ));
                    }
                }
            }
        }
        stats_list
    }

    async fn collect_wifi_stats_async(&self) -> Option<netsim_proto::stats::WifiStats> {
        let client = self.chip_clients.get(&netsim_model::ChipKind::WIFI)?;
        match tokio::time::timeout(CHIP_READ_TIMEOUT, client.get_global_stats()).await {
            Ok(Ok(Some(stats_bytes))) => {
                match netsim_proto::stats::WifiStats::parse_from_bytes(&stats_bytes) {
                    Ok(stats) => Some(stats),
                    Err(e) => {
                        error!("DeviceActor: Failed to parse WifiStats: {}", e);
                        None
                    }
                }
            }
            Ok(Ok(None)) => None,
            Ok(Err(e)) => {
                warn!("DeviceActor: Failed to get global stats: {}", e);
                None
            }
            Err(_) => {
                debug!("DeviceActor: Timeout getting global stats");
                None
            }
        }
    }

    /// Resolves the specific RadioKind for a chip, handling ambiguous cases
    /// like Bluetooth. Fetches fresh chip state if necessary to distinguish
    /// between LE and Classic.
    async fn resolve_radio_kind(&self, chip: &Chip) -> Option<netsim_model::stats::RadioKind> {
        if chip.kind != netsim_model::ChipKind::BLUETOOTH {
            return Some(netsim_model::chip::chip_kind_to_radio_kind(chip.kind));
        }

        // Try to get fresh chip state
        let fresh_chip = if let Some(client) = self.chip_clients.get(&chip.kind) {
            tokio::time::timeout(CHIP_READ_TIMEOUT, client.read(ChipId(chip.id)))
                .await
                .ok()
                .and_then(|res| res.ok())
        } else {
            None
        };

        let chip = fresh_chip.as_ref().unwrap_or(chip);
        Self::resolve_bluetooth_kind(chip, Some(chip))
    }

    fn resolve_bluetooth_kind(
        chip: &Chip,
        state: Option<&Chip>,
    ) -> Option<netsim_model::stats::RadioKind> {
        let chip_state = state.unwrap_or(chip);
        match (chip_state.is_le_enabled(), chip_state.is_classic_enabled()) {
            (true, false) => Some(netsim_model::stats::RadioKind::BluetoothLowEnergy),
            (false, true) => Some(netsim_model::stats::RadioKind::BluetoothClassic),
            _ => None, /* Ambiguous (Dual Mode) or Invalid -> Drop
                        * TODO: Requires HCI packet inspection to accurately distinguish traffic */
        }
    }

    async fn perform_device_deletion(
        &mut self,
        id: DeviceId,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), DeviceError> {
        let stats_to_archive = {
            let Some(internal_device) = self.devices.get(&id) else {
                return Err(DeviceError::DeviceNotFound(id.to_string()));
            };

            let chips = internal_device.device.chips.clone();
            let mut stats_to_archive = Vec::new();
            for chip in &chips {
                let chip_id = ChipId(chip.id);
                let stream_stats = internal_device.chip_stats.get(&chip_id).cloned();
                let cached_stats = internal_device.last_known_stats.get(&chip_id).cloned();
                stats_to_archive.push((chip.clone(), stream_stats, cached_stats));
            }
            stats_to_archive
        };

        for (chip, stream_stats, cached_stats) in stats_to_archive {
            let chip_id = ChipId(chip.id);
            self.archive_chip_stats(id, &chip, stream_stats, cached_stats.as_ref()).await;

            if let Some(chip_client) = self.chip_clients.get(&chip.kind) {
                chip_client.delete(chip_id).await?;
            }

            // Notify Link Actor
            self.link_client
                .notify_chip_removed(chip_id)
                .await
                .expect("Failed to notify LinkActor of chip remove");
        }

        if let Some(internal_device) = self.devices.remove(&id) {
            if let Some(guid) = &internal_device.guid {
                self.guid_to_id.remove(guid);
            }
        }
        self.stats.update_device_count(self.devices.len(), false);
        self.update_idle_state(ctx);
        self.save_stats_async().await;
        Ok(())
    }

    async fn perform_chip_removal(
        &mut self,
        device_id: DeviceId,
        chip_id: ChipId,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), DeviceError> {
        // Find and clone the chip + stats for archiving
        let chip_to_archive = if let Some(entity) = self.devices.get(&device_id) {
            entity.device.chips.iter().find(|c| c.id == chip_id.0).map(|c| {
                let stream_stats = entity.chip_stats.get(&ChipId(c.id)).cloned();
                let cached_stats = entity.last_known_stats.get(&ChipId(c.id)).cloned();
                (c.clone(), stream_stats, cached_stats)
            })
        } else {
            None
        };

        // Archive stats (if found)
        if let Some((chip, stream_stats, cached_stats)) = chip_to_archive {
            self.archive_chip_stats(device_id, &chip, stream_stats, cached_stats.as_ref()).await;
        }

        // Remove the chip and check for deletion
        let should_delete = {
            let entity = self
                .devices
                .get_mut(&device_id)
                .ok_or_else(|| DeviceError::DeviceNotFound(device_id.to_string()))?;
            entity.device.chips.retain(|c| c.id != chip_id.0);
            entity.device.chips.is_empty()
        };

        // Notify Link Actor
        self.link_client
            .notify_chip_removed(chip_id)
            .await
            .expect("Failed to notify LinkActor of chip remove");

        if should_delete {
            info!("DeviceActor: Device {} is empty, auto-deleting", device_id);
            self.handle_delete(device_id, ctx).await?;
        } else {
            // Save stats if we didn't delete the device (partial update)
            self.save_stats_async().await;
            info!("DeviceActor: Device {} is NOT empty after chip removal", device_id);
        }

        Ok(())
    }

    async fn archive_chip_stats(
        &mut self,
        device_id: DeviceId,
        chip: &Chip,
        stream_stats: Option<Arc<StreamStats>>,
        cached_stats: Option<&Vec<netsim_model::stats::NetsimRadioStats>>,
    ) {
        debug!("DeviceActor: Archiving stats for device {} chip {}", device_id, chip.id);

        let mut archive_stats = |mut chip_stats: Vec<netsim_model::stats::NetsimRadioStats>| {
            if let Some(stream_stats) = &stream_stats {
                crate::utils::distribute_stream_stats(&mut chip_stats, stream_stats);
            }
            for radio_stats in chip_stats {
                self.stats.archive(to_proto_stats(radio_stats));
            }
        };

        if let Some(client) = self.chip_clients.get(&chip.kind) {
            if let Ok(Ok(stats)) =
                tokio::time::timeout(CHIP_READ_TIMEOUT, client.read_statistics()).await
            {
                let chip_stats: Vec<_> = stats
                    .into_vec()
                    .into_iter()
                    .filter(|s| s.id == chip.id)
                    .map(|mut s| {
                        s.id = device_id.0.into();
                        s.duration_secs =
                            stream_stats.as_ref().map_or(0, |x| x.start_time.elapsed().as_secs());
                        s
                    })
                    .collect();

                if !chip_stats.is_empty() {
                    archive_stats(chip_stats);
                    return;
                }
            }
        }

        if let Some(cached) = cached_stats {
            debug!("DeviceActor: Using cached LKG stats for chip {}", chip.id);
            archive_stats(cached.clone());
            return;
        }

        if let Some(stream_stats) = stream_stats {
            if let Some(kind) = self.resolve_radio_kind(chip).await {
                self.stats.archive(crate::utils::stream_to_proto_stats(
                    device_id.0,
                    kind,
                    stream_stats.start_time.elapsed().as_secs(),
                    &stream_stats,
                ));
            } else {
                warn!(
                    "DeviceActor: Dropping ambiguous stats for chip {} kind {:?}",
                    chip.id, chip.kind
                );
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
        let chip_name = if chip_config.name.is_empty() {
            entity.device.name.clone()
        } else {
            chip_config.name.clone()
        };
        let manufacturer = if chip_config.manufacturer.is_empty() {
            entity
                .device
                .device_info
                .as_ref()
                .map(|info| info.kind.clone())
                .unwrap_or("Unknown".to_string())
        } else {
            chip_config.manufacturer.clone()
        };
        let product_name = if chip_config.product_name.is_empty() {
            entity.device.name.clone()
        } else {
            chip_config.product_name.clone()
        };
        info!(
            "DeviceActor: AddChip {} ({}, {}) to device {}",
            chip_name, manufacturer, product_name, entity.device.name
        );

        let chip_id = ChipId(next_chip_id.fetch_add(1, Ordering::SeqCst));
        let chip_kind_params = chip_config.chip_kind_params.clone();
        let chip_kind = ChipKind::from(&chip_kind_params);

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

        let chip_client = chip_clients.get(&chip_kind).ok_or_else(|| {
            DeviceError::ChipKindNotSupported(format!("No chip client for {:?}", chip_kind))
        })?;

        let chip_create_params = ChipCreate {
            packet_stream,
            packet_sink,
            config: netsim_model::chip::ChipConfig {
                name: chip_name.clone(),
                manufacturer: manufacturer.clone(),
                product_name: product_name.clone(),
                chip_kind_params,
            },
            device_id: DeviceId(entity.device.id),
            pose: entity.device.pose,
        };

        chip_client.create(chip_id, chip_create_params).await?;
        entity.device.chips.push(Chip {
            id: chip_id.0,
            kind: ChipKind::from(&chip_config.chip_kind_params),
            name: chip_name,
            manufacturer,
            product_name,
            pose: entity.device.pose,
            device_id: DeviceId(entity.device.id),
            variant: Some(ChipVariant::from(chip_kind)),
            links: vec![],
            enabled: true,
        });

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
        info!("DeviceActor: Create device {}", params.device_config.name);

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

        if let Some(device_info) = &params.device_config.device_info {
            self.stats.add_device_stats(crate::utils::to_proto_device_stats(id.0, device_info));
        }

        self.update_idle_state(ctx);
        Ok(id)
    }

    /// Adds a chip to a device by GUID, creating the device if necessary.
    async fn perform_add_chip_by_guid(
        &mut self,
        params: DeviceAddChip,
        ctx: &mut DynContext<Self>,
    ) -> Result<DeviceActionResult, DeviceError> {
        info!("DeviceActor: AddChipByGuid for device {}", params.device_guid);

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

            // Trigger immediate save
            self.save_stats_async().await;

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
                .ok_or_else(|| DeviceError::DeviceNotFound("Device created without chips".into()))?
                .device
                .chips
                .first()
                .ok_or_else(|| DeviceError::DeviceNotFound("Device created without chips".into()))?
                .id;

            // Trigger immediate save
            self.save_stats_async().await;

            Ok(DeviceActionResult::AddChipByGuidSuccess { device_id: id, chip_id: ChipId(chip_id) })
        }
    }

    /// Callback for startup timeout
    pub(crate) fn on_startup_timeout(&mut self, ctx: &mut dyn Context<Self>) {
        let has_active_devices = self.devices.values().any(|d| !d.device.builtin);
        if !self.has_seen_device && !has_active_devices {
            info!(
                "DeviceActor: Startup timeout reached (no devices connected), initiating shutdown"
            );
            self.trigger_shutdown(ctx);
        }
        self.startup_timer = None;
    }

    /// Callback for idle timeout
    pub(crate) fn on_idle_timeout(&mut self, ctx: &mut dyn Context<Self>) {
        let has_active_devices = self.devices.values().any(|d| !d.device.builtin);
        if !has_active_devices {
            info!("DeviceActor: Idle timeout reached, initiating shutdown");
            self.trigger_shutdown(ctx);
        }
        self.idle_timer = None;
    }

    /// Helper utility to initiate graceful shutdown, ensuring stats are
    /// flushed.
    fn trigger_shutdown(&mut self, ctx: &mut dyn Context<Self>) {
        let stats_task = self.stats_write_task.take();
        if let Some(client) = self.self_client.clone() {
            tokio::spawn(async move {
                if let Some(t) = stats_task {
                    if let Err(e) = t.await {
                        warn!("DeviceActor: Stats write failed during shutdown flush: {}", e);
                    }
                }
                if let Err(e) = client.shutdown().await {
                    error!("DeviceActor: Failed to shutdown: {}", e);
                }
            });
        } else {
            warn!("DeviceActor: No self_client, forcing immediate shutdown");
            ctx.shutdown();
        }
    }

    async fn perform_reset(&mut self, id: Option<DeviceId>) -> Result<(), DeviceError> {
        let device_ids: Vec<DeviceId> = if let Some(id) = id {
            if !self.devices.contains_key(&id) {
                return Err(DeviceError::DeviceNotFound(id.to_string()));
            }
            vec![id]
        } else {
            self.devices.keys().cloned().collect()
        };

        let mut chip_client_errors = Vec::new();

        for device_id in device_ids {
            let Some(entity) = self.devices.get_mut(&device_id) else {
                error!("DeviceActor: Device {} disappeared during reset", device_id);
                continue;
            };
            info!("DeviceActor: Resetting device {}", entity.device.name);

            // Chip actors do not track creation parameters, so the device actor must issue
            // an update after reset to restore the original position + orientation.
            let (original_pos, original_orient) = entity
                .create_params
                .as_ref()
                .map(|p| (p.device_config.pose.position, p.device_config.pose.orientation))
                .unwrap_or_default();

            entity.device.visible = true;
            entity.device.pose.position = original_pos;
            entity.device.pose.orientation = original_orient;

            for chip in entity.device.chips.iter_mut() {
                chip.pose.position = original_pos;
                chip.pose.orientation = original_orient;

                if let Some(chip_client) = self.chip_clients.get(&chip.kind) {
                    match chip_client.reset(netsim_model::ChipId(chip.id)).await {
                        Ok(updated_chip) => {
                            *chip = updated_chip;
                        }
                        Err(e) => {
                            warn!(
                                "DeviceActor: Failed to reset chip {} kind {:?}: {}",
                                chip.id, chip.kind, e
                            );
                            chip_client_errors.push((chip.id, e));
                        }
                    }
                }
            }
        }

        let link_client_error = if id.is_none() {
            info!("DeviceActor: Resetting all links");
            if let Err(e) = self.link_client.reset().await {
                warn!("DeviceActor: Failed to reset links: {}", e);
                Some(e)
            } else {
                None
            }
        } else {
            None
        };

        self.save_stats_async().await;

        if !chip_client_errors.is_empty() {
            Err(DeviceError::ResetErrors { chip_client_errors, link_client_error })
        } else {
            Ok(())
        }
    }
}

impl ActorService for DeviceActor {
    type Id = DeviceId;
    type Create = DeviceCreate;
    type Update = DeviceUpdate;
    type Action = DeviceAction;
    type ActionResult = DeviceActionResult;
    type Error = DeviceError;
    type Entity = device_api::Device;
    type TypedStream = ();

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
        update.apply(&mut entity.device);

        for chip in entity.device.chips.iter_mut() {
            let Some(chip_client) = self.chip_clients.get(&chip.kind) else {
                continue;
            };

            let mut chip_update = ChipUpdate::default();

            chip_update.pose = update.pose.clone();

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

            if let Some(u) = specific_update {
                if u.variant.is_some() {
                    chip_update.variant = u.variant.clone();
                }
            }

            if chip_update.pose.position.is_some()
                || chip_update.pose.orientation.is_some()
                || chip_update.variant.is_some()
            {
                info!(
                    "DeviceActor: Updating chip {} (kind {:?}) with {:?}",
                    chip.id, chip.kind, chip_update
                );
                *chip = chip_client.update(ChipId(chip.id), chip_update).await?;
            }
        }
        Ok(entity.device.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        self.perform_device_deletion(id, ctx).await
    }

    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        // Handle actions that don't require looking up the entity first
        match action {
            DeviceAction::GetRadioStats => {
                let stats = self.collect_radio_stats_async().await;
                Ok(DeviceActionResult::Statistics(stats))
            }
            DeviceAction::SaveStats => {
                self.save_stats_async().await;
                Ok(DeviceActionResult::Success)
            }
            DeviceAction::Reset => {
                self.perform_reset(id).await?;
                Ok(DeviceActionResult::Success)
            }
            DeviceAction::AddChipByGuid { params } => {
                self.perform_add_chip_by_guid(params, ctx).await
            }
            DeviceAction::NotifyChipRemoved(device_id, chip_id) => {
                // Verify ID match if provided
                if let Some(id) = id {
                    if id != device_id {
                        return Err(DeviceError::NotFound(format!(
                            "Device ID mismatch: {} vs {}",
                            id, device_id
                        )));
                    }
                }

                self.perform_chip_removal(device_id, chip_id, ctx).await?;
                Ok(DeviceActionResult::Success)
            }

            DeviceAction::DeleteDevice(id) => {
                let Some(internal_device) = self.devices.get(&id) else {
                    return Err(DeviceError::NotFound(format!("Device not found: {}", id)));
                };
                if internal_device.guid.is_some() {
                    return Err(DeviceError::NotFound("Cannot delete external device".to_string()));
                }
                self.perform_device_deletion(id, ctx).await?;
                Ok(DeviceActionResult::Success)
            }
            DeviceAction::AddChip { chip_config, packet_stream, packet_sink } => {
                let Some(id) = id else {
                    return Err(DeviceError::NotFound("AddChip requires a device ID".into()));
                };
                let Some(entity) = self.devices.get_mut(&id) else {
                    return Err(DeviceError::DeviceNotFound(id.to_string()));
                };

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
                    Ok(chip_id) => {
                        self.save_stats_async().await;
                        Ok(DeviceActionResult::ChipId(chip_id))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.devices.values().map(|e| e.device.clone()).collect())
    }
}
