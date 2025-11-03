use crate::chips::{ChipConfig, PacketSink, PacketStream};
use crate::client_error::ClientError;
use crate::client_method;
use crate::device_error::DeviceError;
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::{mpsc, oneshot};

// DEVICE SERVICE
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Orientation {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// A unique identifier for a simulated device, represented as a u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(pub u32);

impl From<DeviceId> for u32 {
    fn from(id: DeviceId) -> Self {
        id.0
    }
}

impl From<u32> for DeviceId {
    fn from(id: u32) -> Self {
        DeviceId(id)
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Responder<T> = oneshot::Sender<Result<T, DeviceError>>;

#[derive(Clone)]
pub struct DeviceClient {
    sender: mpsc::Sender<DeviceRequest>,
}

impl DeviceClient {
    /// Creates a new `DeviceClient` handle.
    ///
    /// This function connects the client to the service's message channel.
    ///
    /// # Arguments
    ///
    /// * `sender` - The `mpsc` sender half of the channel for sending `DeviceRequest`s.
    pub fn new(sender: mpsc::Sender<DeviceRequest>) -> Self {
        Self { sender }
    }

    /// Sends a shutdown command to the device service.
    ///
    /// This is a fire-and-forget command; it does not wait for a response.
    pub async fn shutdown(&self) -> Result<(), ClientError> {
        self.sender
            .send(DeviceRequest::Shutdown)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        Ok(())
    }
}

// Generate client methods.
client_method!(DeviceClient => fn create(device: Box<api::DeviceCreate>) -> DeviceId as DeviceRequest::Create);
client_method!(DeviceClient => fn ps_create(params: CreateDeviceParams) -> () as DeviceRequest::PsCreate);
client_method!(DeviceClient => fn list() -> api::ListDeviceResponse as DeviceRequest::List);
client_method!(DeviceClient => fn delete(id: DeviceId) -> () as DeviceRequest::Delete);

#[allow(dead_code)]
pub struct GetVersionMessage {
    response: oneshot::Sender<String>,
}

pub mod api {
    use crate::chips::BleBeacon;
    use crate::devices::{Device, DeviceConfig};
    use serde::{Deserialize, Serialize};

    // TODO: Revisit the APIs to separate the Api from the Domain.

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ListDeviceResponse {
        pub devices: Vec<Device>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct PatchDeviceRequest {
        pub device: Option<Device>,
    }

    /// The top-level parameters for creating any kind of chip.
    #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
    pub struct DeviceCreate {
        pub config: DeviceConfig,
        pub chip: ChipCreate,
    }

    // External API for chip creation.
    #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ChipCreate {
        /// The name of the chip.
        pub name: String,
        /// The manufacturer of the chip.
        pub manufacturer: String,
        /// The product name of the chip.
        pub product_name: String,
        pub chip: Chip,
    }

    impl ChipCreate {
        pub fn new(
            name: impl Into<String>,
            manufacturer: impl Into<String>,
            product_name: impl Into<String>,
            chip: Chip,
        ) -> ChipCreate {
            ChipCreate {
                name: name.into(),
                manufacturer: manufacturer.into(),
                product_name: product_name.into(),
                chip,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum Chip {
        Beacon(BleBeacon),
    }

    impl Default for Chip {
        fn default() -> Self {
            Chip::Beacon(BleBeacon::default())
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub position: Position,
    pub orientation: Orientation,
    pub chips: Vec<crate::chips::Chip>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// The name of the device.
    /// TODO: Decide if we should support device and chip name for accessories.
    pub name: String,
    /// Whether the device is visible in the UI.
    pub visible: bool,
    /// The position of the device in the simulated world.
    pub position: Position,
    /// The orientation of the device.
    pub orientation: Orientation,
}

impl DeviceConfig {
    pub fn new(
        name: impl Into<String>,
        visible: bool,
        position: Position,
        orientation: Orientation,
    ) -> DeviceConfig {
        DeviceConfig { name: name.into(), visible, position, orientation }
    }
}

pub struct CreateDeviceParams {
    pub device_guid: String,
    pub packet_stream: Option<PacketStream>,
    pub packet_sink: Option<PacketSink>,
    pub device_config: DeviceConfig,
    pub chip_config: ChipConfig,
}
impl fmt::Debug for CreateDeviceParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateDeviceParams")
            .field("device_guid", &self.device_guid)
            .field("device_config", &self.device_config)
            .field("chip_config", &self.chip_config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
/// Defines the message protocol for the device service actor.
pub enum DeviceRequest {
    /// Create a new device from PacketStream.
    PsCreate { params: CreateDeviceParams, respond_to: Responder<()> },
    /// Create a new device.
    Create {
        /// The parameters for the new device.
        device: Box<api::DeviceCreate>,
        /// The channel to send the device ID back on.
        respond_to: Responder<DeviceId>,
    },
    /// List all devices.
    List {
        /// The channel to send the list of devices back on.
        respond_to: Responder<api::ListDeviceResponse>,
    },
    /// Update an existing device.
    Update {
        /// The patch to apply to the device.
        request: api::PatchDeviceRequest,
    },
    /// Delete the chip, removing the device if no other chips are attached.
    Delete {
        /// Device Identifier
        id: DeviceId,
        /// The channel to send the list of devices back on.
        respond_to: Responder<()>,
    },
    /// Reset all devices.
    Reset {},
    GetChipStatistics {
        /// The channel to send the list of devices back on.
        respond_to: Responder<()>,
    },
    /// Shutdown device server.
    Shutdown,
}
