use netsim_model::device::api::DeviceCreate;
use netsim_model::device::Device;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntity {
    pub device: Device,
    pub create_params: Option<DeviceCreate>,
}
