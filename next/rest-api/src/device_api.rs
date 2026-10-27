// Copyright 2026 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Device REST API endpoints and data transfer models.

pub use netsim_model::{
    Device, DeviceAddChip, DeviceChipCreate, DeviceConfig, DeviceCreate, DeviceId, DeviceInfo,
    DeviceUpdate, PoseUpdate,
};

pub use crate::ListDeviceResponse;
use crate::{
    client::{NetsimRestClient, RestRequest},
    server::{HttpError, JsonResponse, NoContent, RestServer, json_response},
};

impl From<Vec<Device>> for ListDeviceResponse {
    fn from(devices: Vec<Device>) -> Self {
        Self { devices }
    }
}

impl From<Device> for JsonResponse {
    fn from(device: Device) -> Self {
        Self(json_response(&device))
    }
}

impl From<Vec<Device>> for JsonResponse {
    fn from(devices: Vec<Device>) -> Self {
        Self(json_response(&ListDeviceResponse::from(devices)))
    }
}

impl RestServer {
    pub(crate) async fn list_devices(&self) -> Result<Vec<Device>, HttpError> {
        if let Err(error) = self.device_client.get_radio_stats().await {
            tracing::warn!(%error, "Failed to get radio stats");
        }
        Ok(self.device_client.list().await?)
    }

    pub(crate) async fn reset_devices(&self) -> Result<NoContent, HttpError> {
        self.device_client.reset(None).await?;
        self.ap_client.reset().await?;
        Ok(NoContent)
    }
}

impl NetsimRestClient {
    /// `GET /v1/devices`
    pub fn get_devices(&self) -> RestRequest<'_, ListDeviceResponse> {
        self.get(crate::path::DEVICES)
    }

    /// `POST /v1/devices:reset`
    pub fn reset(&self) -> RestRequest<'_, ()> {
        self.post(format!("{}:reset", crate::path::DEVICES))
    }
}
