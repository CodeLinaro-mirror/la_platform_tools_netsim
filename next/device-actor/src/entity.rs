use device_api::api::DeviceCreate;
use device_api::Device;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntity {
    pub device: Device,
    pub create_params: Option<DeviceCreate>,
}
