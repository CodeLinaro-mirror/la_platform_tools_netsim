// Copyright (C) 2025 The Android Open Source Project

//! Device Client
//!
//! This module provides the [`DeviceClient`] struct, which is a type-safe wrapper
//! around the generic [`ResourceClient`]. It provides a convenient API for
//! interacting with Device actors, including methods for standard operations
//! and custom actions.

use actor_framework::{ActorClient, FrameworkError, ResourceClient};
use async_trait::async_trait;
use device_actor::DeviceActor;

use device_actor::DeviceError;
use device_api::api::DeviceCreate;
use device_api::DeviceId;
use device_api::{DeviceAction, DeviceActionResult};
use log::debug;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct DeviceClient {
    pub(crate) inner: ResourceClient<DeviceActor>,
    pub(crate) state: Arc<Mutex<DeviceClientState>>,
}

#[derive(Default, Debug)]
pub(crate) struct DeviceClientState {
    pub(crate) guid_to_id: HashMap<String, DeviceId>,
}

mod device_add_chip;

impl DeviceClient {
    pub fn new(inner: ResourceClient<DeviceActor>) -> Self {
        Self { inner, state: Arc::new(Mutex::new(DeviceClientState::default())) }
    }
}

#[async_trait]
impl ActorClient<DeviceActor> for DeviceClient {
    type Error = DeviceError;

    fn inner(&self) -> &ResourceClient<DeviceActor> {
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

    pub async fn list(&self) -> Result<device_api::api::ListDeviceResponse, DeviceError> {
        debug!("Sending list devices request");
        let devices = self
            .inner
            .list()
            .await
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;
        Ok(device_api::api::ListDeviceResponse { devices })
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
    pub async fn reset(&self, id: DeviceId) -> Result<(), DeviceError> {
        debug!("Sending reset request for device {}", id);
        match self.inner.perform_action(Some(id), DeviceAction::Reset).await {
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
}
