// Copyright (C) 2025 The Android Open Source Project

//! Device Client
//!
//! This module provides the [`DeviceClient`] struct, which is a
//! `Box<dyn ActorClient<DeviceActor>>`.
//! It provides a convenient API for interacting with Device actors,
//! including methods for standard operations and custom actions.

use crate::DeviceActor;
use crate::DeviceError;
use actor_framework::ActorClient;
use device_api::api::DeviceCreate;
use device_api::DeviceId;
use device_api::{DeviceAction, DeviceActionResult};
use log::debug;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DeviceClient {
    pub(crate) inner: Box<dyn ActorClient<DeviceActor>>,
    pub(crate) state: Arc<Mutex<DeviceClientState>>,
}

impl std::fmt::Debug for DeviceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceClient").field("state", &self.state).finish_non_exhaustive()
    }
}

#[derive(Default, Debug)]
pub(crate) struct DeviceClientState {
    pub(crate) guid_to_id: HashMap<String, DeviceId>,
}

mod device_add_chip;

impl DeviceClient {
    pub fn new(inner: Box<dyn ActorClient<DeviceActor>>) -> Self {
        Self { inner, state: Arc::new(Mutex::new(DeviceClientState::default())) }
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

    /// Gets a device by ID.
    pub async fn get(&self, id: DeviceId) -> Result<Option<device_api::Device>, DeviceError> {
        debug!("Sending get request for device {}", id);
        self.inner.get(id).await.map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    pub async fn list(&self) -> Result<device_api::api::ListDeviceResponse, DeviceError> {
        debug!("Sending list devices request");
        let devices = self
            .inner
            .list()
            .await
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
        Ok(device_api::api::ListDeviceResponse { devices })
    }

    /// Resolves a device ID from a name.
    ///
    /// This method lists all devices and finds the one with the matching name.
    /// If multiple devices have the same name, it returns the first one found.
    pub async fn resolve_id_by_name(&self, name: &str) -> Result<DeviceId, DeviceError> {
        debug!("Resolving device ID for name: {}", name);
        let list_resp = self.list().await?;
        list_resp
            .devices
            .into_iter()
            .find(|d| d.name == name)
            .map(|d| DeviceId(d.id))
            .ok_or_else(|| DeviceError::NotFound(name.to_string()))
    }

    /// Patches a device by ID or Name.
    ///
    /// If `id` is provided (non-zero), it is used.
    /// Otherwise, `name` is used to resolve the ID.
    pub async fn patch(
        &self,
        id: Option<u32>,
        name: Option<&str>,
        mut update: device_api::api::DeviceUpdate,
    ) -> Result<(), DeviceError> {
        let device_id = if let Some(id) = id.filter(|&v| v != 0) {
            DeviceId(id)
        } else if let Some(name) = name {
            self.resolve_id_by_name(name).await?
        } else {
            return Err(DeviceError::ActorCommunicationError(
                "Device ID or Name must be provided".to_string(),
            ));
        };
        update.id = device_id.0;
        self.update(device_id, update).await
    }

    /// Updates an existing device's properties.
    ///
    /// # Arguments
    /// * `id` - The ID of the device to update.
    /// * `update` - The update parameters.
    pub async fn update(
        &self,
        id: DeviceId,
        update: device_api::api::DeviceUpdate,
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
    /// If `id` is `None`, it performs a global reset.
    pub async fn reset(&self, id: Option<DeviceId>) -> Result<(), DeviceError> {
        debug!("Sending reset request for device {:?}", id);
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
        match self
            .inner
            .perform_action(Some(id), DeviceAction::NotifyChipRemoved(id, chip_id))
            .await
        {
            Ok(DeviceActionResult::Success) => Ok(()),
            Ok(_) => {
                Err(DeviceError::ActorCommunicationError("Unexpected action result".to_string()))
            }
            Err(e) => Err(DeviceError::ActorCommunicationError(e.to_string())),
        }
    }
    /// Deletes a device.
    ///
    /// # Arguments
    /// * `id` - The ID of the device to delete.
    pub async fn delete(&self, id: DeviceId) -> Result<(), DeviceError> {
        debug!("Sending delete request for device {}", id);
        self.inner.delete(id).await.map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }
}
