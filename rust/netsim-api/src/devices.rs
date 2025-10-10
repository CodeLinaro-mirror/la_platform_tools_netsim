use netsim_proto::frontend::{ListDeviceResponse, PatchDeviceRequest};
use netsim_proto::model::Device;
use tokio::sync::oneshot;

// DEVICE SERVICE

pub enum DeviceCommand {
    CreateDevice { device: Device, responder: oneshot::Sender<Device> },
    PatchDevice { request: PatchDeviceRequest },
    ListDevice { responder: oneshot::Sender<ListDeviceResponse> },
}

#[allow(dead_code)]
pub struct GetVersionMessage {
    response: oneshot::Sender<String>,
}

/// The parameters for creating a Bluetooth virtual device.
#[derive(Debug, Clone, Default)]
pub struct BluetoothDeviceParams {
    pub address: String,
    // TODO: Add rootcanal controller properties when the dependency is available.
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseData.Service`
#[derive(Debug, Clone, Default)]
pub struct Service {
    pub uuid: String,
    pub data: Vec<u8>,
}
