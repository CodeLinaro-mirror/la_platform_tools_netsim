use crate::device_actor::DeviceActor;
use crate::error::DeviceError;
use crate::utils::{chip_kind_to_network_kind, create_capture_and_wrap_streams};
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use device_api::api::{DeviceCreate, DeviceUpdate};
use device_api::{DeviceAction, DeviceActionResult, DeviceId};
use link_api::LinkAction;
use netsim_model::chip::{ChipConfig, ChipCreate, ChipId, NetworkKind, NetworkParams};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct InternalDevice {
    pub device: device_api::Device,
    pub create_params: Option<DeviceCreate>,
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
                chips: vec![],
            },
            create_params: Some(params),
        })
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
        ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        log::info!("DeviceActor: handle_create for device {}", params.device_config.name);
        self.has_seen_device = true;
        self.last_empty_time = None;
        let id = id.unwrap_or_else(|| {
            let id = DeviceId(self.next_device_id);
            self.next_device_id += 1;
            id
        });
        let mut entity = InternalDevice::from_create_params(id, params)?;

        if let Some(params) = entity.create_params.take() {
            let chip_id = ChipId(self.next_chip_id.fetch_add(1, Ordering::SeqCst));
            let chip_create = params.chip;

            // 1. Create Chip parameters
            let network_params: NetworkParams = chip_create.chip.into();
            let chip_kind = NetworkKind::from(&network_params);

            let chip_params = ChipCreate {
                id: chip_id,
                packet_stream: None,
                packet_sink: None,
                config: ChipConfig {
                    name: chip_create.name.clone(),
                    manufacturer: chip_create.manufacturer.clone(),
                    product_name: chip_create.product_name.clone(),
                    network_params,
                },
                device_id: DeviceId(entity.device.id),
            };

            // 2. Send create request to Chip Actor
            if let Some(chip_client) = self.chip_clients.get(&chip_kind) {
                chip_client
                    .create(chip_params)
                    .await
                    .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

                // 3. Update local device state with the new chip
                entity.device.chips.push(netsim_model::chip::Chip {
                    id: chip_id.0,
                    kind: netsim_model::chip::ChipKind::from(chip_kind),
                    name: Some(chip_create.name),
                    manufacturer: Some(chip_create.manufacturer),
                    product_name: Some(chip_create.product_name),
                    position: entity.device.position.clone(),
                    orientation: entity.device.orientation.clone(),
                    device_id: DeviceId(entity.device.id),
                    variant: None,
                    links: vec![],
                });
                // Send create request to Link Actor
                // This ensures the LinkActor is aware of the new chip and can manage its links.
                self.link_client
                    .action(None, LinkAction::NotifyChipAdded(chip_id, chip_kind.into()))
                    .await
                    .expect("Failed to notify LinkActor of new chip");
                // TODO: Expose a helper method on LinkClient for this action (e.g. notify_chip_added)
                // for correctness, brevity and clarity.
            } else {
                // Log warning or return error if no client for this network kind
                return Err(DeviceError::ActorCommunicationError(format!(
                    "No chip client for {:?}",
                    chip_kind
                )));
            }
        }
        self.devices.insert(id, entity);
        Ok(id)
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
        if let Some(entity) = self.devices.get_mut(&id) {
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
            for chip in &entity.device.chips {
                let network_kind = chip_kind_to_network_kind(&chip.kind);
                if let Some(chip_client) = self.chip_clients.get(&network_kind) {
                    let mut chip_update = netsim_model::chip::ChipUpdate::default();
                    chip_update.position = Some(entity.device.position.clone());
                    chip_update.orientation = Some(entity.device.orientation.clone());

                    chip_client
                        .update(netsim_model::chip::ChipId(chip.id), chip_update)
                        .await
                        .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
                    //TODO: overwrite chip links if update.links is Some
                }
            }
            Ok(entity.device.clone())
        } else {
            Err(DeviceError::DeviceNotFound(id.to_string()))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        // self.last_activity = std::time::Instant::now(); // Removed
        if let Some(entity) = self.devices.remove(&id) {
            for chip in &entity.device.chips {
                let network_kind = chip_kind_to_network_kind(&chip.kind);
                if let Some(chip_client) = self.chip_clients.get(&network_kind) {
                    chip_client
                        .delete(netsim_model::chip::ChipId(chip.id))
                        .await
                        .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
                    // Send delete request to Link Actor
                    self.link_client
                        .action(
                            None,
                            LinkAction::NotifyChipRemoved(netsim_model::chip::ChipId(chip.id)),
                        )
                        .await
                        .expect("Failed to notify LinkActor of chip remove");
                    // TODO: Expose a helper method on LinkClient for this action (e.g. notify_chip_removed)
                }
            }
            if self.devices.is_empty() {
                self.last_empty_time = Some(std::time::Instant::now());
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
        ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        if let Some(id) = id {
            if let Some(mut entity) = self.devices.remove(&id) {
                // Inline handle_action logic
                let result = match action {
                    DeviceAction::Reset => {
                        // TODO: Implement device reset logic if needed
                        Ok(DeviceActionResult::Success)
                    }
                    DeviceAction::NotifyChipRemoved(_device_id, chip_id) => {
                        entity.device.chips.retain(|c| c.id != chip_id.0);
                        if entity.device.chips.is_empty() {
                            // TODO: If entity.device.chips.is_empty(), remove the device itself.
                            // This requires a way to trigger a self-delete from within the actor.
                        }
                        self.link_client
                            .action(None, LinkAction::NotifyChipRemoved(chip_id))
                            .await
                            .expect("Failed to notify LinkActor of chip remove");
                        // TODO: Expose a helper method on LinkClient for this action (e.g. notify_chip_removed)
                        Ok(DeviceActionResult::Success)
                    }
                    DeviceAction::AddChip { chip_config, packet_stream, packet_sink } => {
                        log::info!("DeviceActor: AddChip for chip {}", chip_config.name);
                        let chip_id = ChipId(self.next_chip_id.fetch_add(1, Ordering::SeqCst));

                        // 1. Create Chip parameters
                        let network_params: NetworkParams = chip_config.chip.into();
                        let chip_kind = NetworkKind::from(&network_params);

                        // 2. Handle Capture Creation and Stream Wrapping
                        let (packet_stream, packet_sink) =
                            if let Some(capture_client) = &self.capture_client {
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

                        let chip_params = ChipCreate {
                            id: chip_id,
                            packet_stream,
                            packet_sink,
                            config: netsim_model::chip::ChipConfig {
                                name: chip_config.name.clone(),
                                manufacturer: chip_config.manufacturer.clone(),
                                product_name: chip_config.product_name.clone(),
                                network_params,
                            },
                            device_id: DeviceId(entity.device.id),
                        };

                        // 2. Send create request to Chip Actor
                        if let Some(chip_client) = self.chip_clients.get(&chip_kind) {
                            chip_client
                                .create(chip_params)
                                .await
                                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

                            // 3. Update local device state with the new chip
                            entity.device.chips.push(netsim_model::chip::Chip {
                                id: chip_id.0,
                                kind: netsim_model::chip::ChipKind::from(chip_kind),
                                name: Some(chip_config.name),
                                manufacturer: Some(chip_config.manufacturer),
                                product_name: Some(chip_config.product_name),
                                position: entity.device.position.clone(),
                                orientation: entity.device.orientation.clone(),
                                device_id: DeviceId(entity.device.id),
                                variant: None,
                                links: vec![],
                            });
                            self.link_client
                                .action(
                                    None,
                                    LinkAction::NotifyChipAdded(chip_id, chip_kind.into()),
                                )
                                .await
                                .expect("Failed to notify LinkActor of chip add");
                            // TODO: Expose a helper method on LinkClient for this action (e.g. notify_chip_added)
                            Ok(DeviceActionResult::ChipId(chip_id))
                        } else {
                            Err(DeviceError::ActorCommunicationError(format!(
                                "No chip client for {:?}",
                                chip_kind
                            )))
                        }
                    }
                };

                self.devices.insert(id, entity);
                self.has_seen_device = true;
                self.last_empty_time = None;
                result
            } else {
                Err(DeviceError::DeviceNotFound(id.to_string()))
            }
        } else {
            // Global actions
            match action {
                DeviceAction::Reset => {
                    // TODO: Implement global reset logic
                    Ok(DeviceActionResult::Success)
                }
                _ => Err(DeviceError::NotFound("Action requires a device ID".into())),
            }
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.devices.values().map(|e| e.device.clone()).collect())
    }
}
