// Copyright (C) 2025 The Android Open Source Project

//! Device Client
//!
//! This module provides the [`DeviceClient`] struct, which is a type-safe wrapper
//! around the generic [`ResourceClient`]. It provides a convenient API for
//! interacting with Device actors, including methods for standard operations
//! and custom actions.

use actor_framework::{ActorClient, FrameworkError, ResourceClient};
use async_trait::async_trait;
use device_actor::entity::DeviceEntity;
use device_actor::{DeviceAction, DeviceActionResult, DeviceError};
use log::debug;
use netsim_model::device::api::DeviceCreate;
use netsim_model::device::DeviceId;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct DeviceClient {
    inner: ResourceClient<DeviceEntity>,
    state: Arc<Mutex<DeviceClientState>>,
}

#[derive(Default, Debug)]
struct DeviceClientState {
    guid_to_id: HashMap<String, DeviceId>,
}

impl DeviceClient {
    pub fn new(inner: ResourceClient<DeviceEntity>) -> Self {
        Self { inner, state: Arc::new(Mutex::new(DeviceClientState::default())) }
    }
}

#[async_trait]
impl ActorClient<DeviceEntity> for DeviceClient {
    type Error = DeviceError;

    fn inner(&self) -> &ResourceClient<DeviceEntity> {
        &self.inner
    }

    fn map_error(e: FrameworkError) -> Self::Error {
        DeviceError::ActorCommunicationError(e.to_string())
    }
}

impl DeviceClient {
    /// Creates a new device with the given parameters.
    ///
    /// This is a standard creation method that delegates to the underlying
    /// `ResourceClient::create` method.
    pub async fn create_device(&self, params: DeviceCreate) -> Result<DeviceId, DeviceError> {
        debug!("Sending create device request");
        self.inner
            .create(params)
            .await
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    pub async fn list(&self) -> Result<netsim_model::device::api::ListDeviceResponse, DeviceError> {
        debug!("Sending list devices request");
        self.inner.list().await.map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    /// Updates an existing device's properties.
    ///
    /// # Arguments
    /// * `id` - The ID of the device to update.
    /// * `update` - The update parameters.
    pub async fn update(
        &self,
        id: DeviceId,
        update: netsim_model::device::api::DeviceUpdate,
    ) -> Result<(), DeviceError> {
        debug!("Sending update request for device {}", id);
        self.inner
            .update(id, update)
            .await
            .map(|_| ())
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    /// Resets a device to its default state.
    ///
    /// This sends a `DeviceAction::Reset` to the device actor.
    pub async fn reset(&self, id: DeviceId) -> Result<(), DeviceError> {
        debug!("Sending reset request for device {}", id);
        match self.inner.perform_action(id, DeviceAction::Reset).await {
            Ok(DeviceActionResult::Success) => Ok(()),
            Ok(_) => {
                Err(DeviceError::ActorCommunicationError("Unexpected action result".to_string()))
            }
            Err(e) => Err(DeviceError::ActorCommunicationError(e.to_string())),
        }
    }

    /// Notifies the device that one of its chips has been removed.
    ///
    /// This is used to maintain consistency between the device and its chips.
    pub async fn notify_chip_removed(
        &self,
        id: DeviceId,
        chip_id: netsim_model::chip::ChipId,
    ) -> Result<(), DeviceError> {
        debug!("Sending notify_chip_removed request for device {} chip {}", id, chip_id);
        match self.inner.perform_action(id, DeviceAction::NotifyChipRemoved(id, chip_id)).await {
            Ok(DeviceActionResult::Success) => Ok(()),
            Ok(_) => {
                Err(DeviceError::ActorCommunicationError("Unexpected action result".to_string()))
            }
            Err(e) => Err(DeviceError::ActorCommunicationError(e.to_string())),
        }
    }

    /// Creates or updates a device based on PacketStream parameters.
    ///
    /// This method handles the logic for PacketStream-based device creation, which
    /// includes checking for existing devices by GUID to support multi-chip devices.
    ///
    /// # How it works
    /// 1. Checks if a device with the given `device_guid` already exists in the local state.
    /// 2. If it exists, it adds a new chip to that device using `DeviceAction::AddChip`.
    /// 3. If it doesn't exist, it creates a new device and stores the mapping from GUID to the new Device ID.
    ///
    /// This ensures that multiple PacketStream connections with the same GUID are grouped under a single device.
    pub async fn add_chip(
        &self,
        params: netsim_model::device::DeviceAddChip,
    ) -> Result<DeviceId, DeviceError> {
        debug!("Processing add_chip for GUID {}", params.device_guid);

        let existing_id = {
            let state = self
                .state
                .lock()
                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            state.guid_to_id.get(&params.device_guid).copied()
        };

        if let Some(device_id) = existing_id {
            // If device already existed, add a new chip
            debug!("Adding chip to existing device {}", device_id);
            match self
                .inner
                .perform_action(
                    device_id,
                    DeviceAction::AddChip(convert_chip_config(&params.chip_config)),
                )
                .await
            {
                Ok(DeviceActionResult::ChipId(_)) => Ok(device_id),
                Ok(_) => Err(DeviceError::ActorCommunicationError(
                    "Unexpected action result".to_string(),
                )),
                Err(e) => Err(DeviceError::ActorCommunicationError(e.to_string())),
            }
        } else {
            // Create new device
            let create_params = DeviceCreate {
                device_config: params.device_config.clone(),
                chip: convert_chip_config(&params.chip_config),
            };
            let id = self.create_device(create_params).await?;

            let mut state = self
                .state
                .lock()
                .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
            state.guid_to_id.insert(params.device_guid, id);
            Ok(id)
        }
    }
}

fn convert_chip_config(
    c: &netsim_model::chip::ChipConfig,
) -> netsim_model::device::api::DeviceChipCreate {
    netsim_model::device::api::DeviceChipCreate {
        name: c.name.clone(),
        manufacturer: c.manufacturer.clone(),
        product_name: c.product_name.clone(),
        chip: netsim_model::device::api::Chip::default(), // TODO: Proper conversion if needed
    }
}
