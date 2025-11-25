// Copyright (C) 2025 The Android Open Source Project

//! Device Entity and Context
//!
//! This module defines the [`DeviceEntity`] which implements the [`ActorEntity`] trait,
//! and the [`DeviceContext`] which provides external dependencies (like Chip Clients)
//! to the actor.
//!
//! The [`DeviceEntity`] manages the state of a simulated device and its associated chips.
//! It handles lifecycle events like creation, update, and deletion, propagating
//! relevant changes to the associated chip actors.

use crate::actions::{DeviceAction, DeviceActionResult};
use crate::error::DeviceError;
use actor_framework::ActorEntity;
use async_trait::async_trait;
use netsim_model::chip::{
    BeaconParams, BluetoothCreate, BluetoothMode, ChipClient, ChipConfig, ChipCreate, ChipId,
    NetworkKind, NetworkParams,
};
use netsim_model::device::api::{Chip, DeviceCreate, DeviceUpdate};
use netsim_model::device::{Device, DeviceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntity {
    pub device: Device,
    pub create_params: Option<DeviceCreate>,
}

pub struct DeviceContext {
    pub chip_clients: HashMap<NetworkKind, ChipClient>,
    pub next_chip_id: Arc<AtomicU32>,
}

#[async_trait]
impl ActorEntity for DeviceEntity {
    type Id = DeviceId;
    type Create = DeviceCreate;
    type Update = DeviceUpdate;
    type Action = DeviceAction;
    type ActionResult = DeviceActionResult;
    type Context = DeviceContext;
    type Error = DeviceError;
    type ListResponse = netsim_model::device::api::ListDeviceResponse;

    /// Creates a new Device from creation parameters.
    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error> {
        Ok(DeviceEntity {
            device: Device {
                id: id.0,
                name: params.device_config.name.clone(),
                visible: params.device_config.visible,
                position: params.device_config.position.clone(),
                orientation: params.device_config.orientation.clone(),
                chips: vec![], // Will be filled in on_create
            },
            create_params: Some(params),
        })
    }

    /// Called after the entity is created.
    /// This is where we handle side effects of creation, like creating associated chips.
    async fn on_create(&mut self, ctx: &Self::Context) -> Result<(), Self::Error> {
        if let Some(params) = self.create_params.take() {
            let chip_id = ChipId(ctx.next_chip_id.fetch_add(1, Ordering::SeqCst));
            let chip_create = params.chip;

            // 1. Create Chip parameters
            let network_params = match &chip_create.chip {
                Chip::Beacon(beacon) => NetworkParams::Bluetooth(BluetoothCreate {
                    address: beacon.address.clone(),
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Beacon(Box::new(BeaconParams {
                        ble_beacon: beacon.clone(),
                    })),
                }),
            };
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
                device_id: DeviceId(self.device.id),
            };

            // 2. Send create request to Chip Actor
            if let Some(chip_client) = ctx.chip_clients.get(&chip_kind) {
                chip_client
                    .create(chip_params)
                    .await
                    .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

                // 3. Update local device state with the new chip
                self.device.chips.push(netsim_model::chip::Chip {
                    id: chip_id.0,
                    kind: netsim_model::chip::ChipKind::from(chip_kind),
                    name: Some(chip_create.name),
                    manufacturer: Some(chip_create.manufacturer),
                    product_name: Some(chip_create.product_name),
                    position: self.device.position.clone(),
                    orientation: self.device.orientation.clone(),
                    device_id: DeviceId(self.device.id),
                    variant: None,
                });
            } else {
                // Log warning or return error if no client for this network kind
                return Err(DeviceError::ActorCommunicationError(format!(
                    "No chip client for {:?}",
                    chip_kind
                )));
            }
        }
        Ok(())
    }

    /// Handles custom actions for the Device actor.
    async fn handle_action(
        &mut self,
        action: Self::Action,
        ctx: &Self::Context,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            DeviceAction::Reset => {
                // TODO: Implement device reset logic if needed
                Ok(DeviceActionResult::Success)
            }
            DeviceAction::NotifyChipRemoved(_device_id, chip_id) => {
                self.device.chips.retain(|c| c.id != chip_id.0);
                // TODO: If self.device.chips.is_empty(), remove the device itself.
                // This requires a way to trigger a self-delete from within the actor.
                Ok(DeviceActionResult::Success)
            }
            DeviceAction::AddChip(chip_config) => {
                let chip_id = ChipId(ctx.next_chip_id.fetch_add(1, Ordering::SeqCst));

                // 1. Create Chip parameters
                let network_params = match &chip_config.chip {
                    Chip::Beacon(beacon) => NetworkParams::Bluetooth(BluetoothCreate {
                        address: beacon.address.clone(),
                        bt_properties: Default::default(),
                        mode: BluetoothMode::Beacon(Box::new(BeaconParams {
                            ble_beacon: beacon.clone(),
                        })),
                    }),
                };
                let chip_kind = NetworkKind::from(&network_params);

                let chip_params = ChipCreate {
                    id: chip_id,
                    packet_stream: None,
                    packet_sink: None,
                    config: netsim_model::chip::ChipConfig {
                        name: chip_config.name.clone(),
                        manufacturer: chip_config.manufacturer.clone(),
                        product_name: chip_config.product_name.clone(),
                        network_params,
                    },
                    device_id: DeviceId(self.device.id),
                };

                // 2. Send create request to Chip Actor
                if let Some(chip_client) = ctx.chip_clients.get(&chip_kind) {
                    chip_client
                        .create(chip_params)
                        .await
                        .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

                    // 3. Update local device state with the new chip
                    self.device.chips.push(netsim_model::chip::Chip {
                        id: chip_id.0,
                        kind: netsim_model::chip::ChipKind::from(chip_kind),
                        name: Some(chip_config.name),
                        manufacturer: Some(chip_config.manufacturer),
                        product_name: Some(chip_config.product_name),
                        position: self.device.position.clone(),
                        orientation: self.device.orientation.clone(),
                        device_id: DeviceId(self.device.id),
                        variant: None,
                    });
                    Ok(DeviceActionResult::ChipId(chip_id))
                } else {
                    Err(DeviceError::ActorCommunicationError(format!(
                        "No chip client for {:?}",
                        chip_kind
                    )))
                }
            }
        }
    }

    /// Called when the entity is updated.
    /// Propagates relevant updates (like position/orientation) to associated chips.
    async fn on_update(
        &mut self,
        update: Self::Update,
        ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        // Update local state
        if let Some(name) = update.name {
            self.device.name = name;
        }
        if let Some(visible) = update.visible {
            self.device.visible = visible;
        }
        if let Some(pos) = update.position {
            self.device.position = pos;
        }
        if let Some(orient) = update.orientation {
            self.device.orientation = orient;
        }

        // Propagate updates to chips
        for chip in &self.device.chips {
            let network_kind = chip_kind_to_network_kind(&chip.kind);
            if let Some(chip_client) = ctx.chip_clients.get(&network_kind) {
                let mut chip_update = netsim_model::chip::ChipUpdate::default();
                chip_update.position = Some(self.device.position.clone());
                chip_update.orientation = Some(self.device.orientation.clone());

                chip_client
                    .update(netsim_model::chip::ChipId(chip.id), chip_update)
                    .await
                    .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Called when the entity is deleted.
    /// Ensures all associated chips are also deleted.
    async fn on_delete(&self, ctx: &Self::Context) -> Result<(), Self::Error> {
        for chip in &self.device.chips {
            let network_kind = chip_kind_to_network_kind(&chip.kind);
            if let Some(chip_client) = ctx.chip_clients.get(&network_kind) {
                chip_client
                    .delete(netsim_model::chip::ChipId(chip.id))
                    .await
                    .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn on_list(entities: &HashMap<Self::Id, Self>) -> Self::ListResponse {
        let devices = entities.values().map(|e| e.device.clone()).collect();
        netsim_model::device::api::ListDeviceResponse { devices }
    }
}

fn chip_kind_to_network_kind(
    kind: &netsim_model::chip::ChipKind,
) -> netsim_model::chip::NetworkKind {
    match kind {
        netsim_model::chip::ChipKind::BLUETOOTH | netsim_model::chip::ChipKind::BleBeacon => {
            netsim_model::chip::NetworkKind::Bluetooth
        }
        netsim_model::chip::ChipKind::WIFI => netsim_model::chip::NetworkKind::Wifi,
        netsim_model::chip::ChipKind::UWB => netsim_model::chip::NetworkKind::Uwb,
        netsim_model::chip::ChipKind::CELLULAR => netsim_model::chip::NetworkKind::Cell,
        _ => netsim_model::chip::NetworkKind::Bluetooth, // Fallback
    }
}
