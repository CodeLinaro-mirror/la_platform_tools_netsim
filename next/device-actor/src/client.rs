// Copyright (C) 2025 The Android Open Source Project

//! Device Client
//!
//! Provides the [`DeviceClient`] struct for interacting with Device actors.

use actor_framework::ActorClient;
use device_api::{api::DeviceCreate, DeviceAction, DeviceActionResult, DeviceId};
use tracing::debug;

use crate::{DeviceActor, DeviceError};

#[derive(Clone)]
pub struct DeviceClient {
    pub(crate) inner: Box<dyn ActorClient<DeviceActor>>,
}

impl std::fmt::Debug for DeviceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceClient").finish_non_exhaustive()
    }
}

impl DeviceClient {
    pub fn new(inner: Box<dyn ActorClient<DeviceActor>>) -> Self {
        Self { inner }
    }
}

impl DeviceClient {
    /// Creates a new device with the given parameters.
    pub async fn create_device(&self, params: DeviceCreate) -> Result<DeviceId, DeviceError> {
        debug!("Sending create device request");
        self.inner
            .create(params)
            .await
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

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

    /// Resets a device to its default state. If `id` is `None`, performs a
    /// global reset.
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
        chip_id: netsim_model::ChipId,
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
    /// Fetches the latest radio statistics from all associated chip clients.
    pub async fn get_radio_stats(
        &self,
    ) -> Result<Vec<netsim_model::stats::NetsimRadioStats>, DeviceError> {
        self.inner
            .perform_action(None, DeviceAction::GetRadioStats)
            .await
            .map(|res| match res {
                DeviceActionResult::Statistics(stats) => stats,
                _ => vec![],
            })
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    /// Deletes a device.
    pub async fn delete(&self, id: DeviceId) -> Result<(), DeviceError> {
        debug!("Sending delete request for device {}", id);
        self.inner.delete(id).await.map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    /// Creates or updates a device based on PacketStream parameters.
    ///
    /// Uses `AddChipByGuid` to atomically find/create a device by GUID.
    pub async fn add_chip(
        &self,
        params: netsim_model::device::DeviceAddChip,
    ) -> Result<DeviceId, DeviceError> {
        let result = self
            .inner
            .perform_action(
                None, // Global action
                DeviceAction::AddChipByGuid { params },
            )
            .await
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))?;

        match result {
            DeviceActionResult::AddChipByGuidSuccess { device_id, chip_id: _ } => Ok(device_id),
            _ => Err(DeviceError::ActorCommunicationError(
                "Unexpected action result for AddChipByGuid".to_string(),
            )),
        }
    }

    /// Shuts down the device actor.
    pub async fn shutdown(&self) -> Result<(), DeviceError> {
        debug!("Sending shutdown request");
        self.inner.shutdown().await.map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }

    /// Triggers a persistence of the current statistics to disk.
    pub async fn save_stats(&self) -> Result<(), DeviceError> {
        self.inner
            .perform_action(None, DeviceAction::SaveStats)
            .await
            .map(|_| ())
            .map_err(|e| DeviceError::ActorCommunicationError(e.to_string()))
    }
}
