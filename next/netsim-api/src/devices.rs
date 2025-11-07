use crate::chips::{ChipConfig, PacketSink, PacketStream};
use crate::client_error::ClientError;
use crate::client_method;
use crate::device_error::DeviceError;
use netsim_proto::frontend::ListDeviceResponse;
use netsim_proto::frontend::PatchDeviceRequest;
use netsim_proto::model::{Orientation as ProtoOrientation, Position as ProtoPosition};
use std::fmt;
use tokio::sync::{mpsc, oneshot};

// DEVICE SERVICE

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
client_method!(DeviceClient => fn list() -> ListDeviceResponse as DeviceRequest::List);
client_method!(DeviceClient => fn delete(id: DeviceId) -> () as DeviceRequest::Delete);

#[allow(dead_code)]
pub struct GetVersionMessage {
    response: oneshot::Sender<String>,
}

pub mod api {
    use crate::chips::BleBeacon;
    use crate::devices::DeviceConfig;

    // TODO: Revisit the APIs to separate the Api from the Domain.

    /// The top-level parameters for creating any kind of chip.
    #[derive(Debug)]
    pub struct DeviceCreate {
        pub config: DeviceConfig,
        pub chip: ChipCreate,
    }

    // External API for chip creation.
    #[derive(Debug)]
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

    #[derive(Debug)]
    pub enum Chip {
        Beacon(BleBeacon),
    }

    #[derive(Debug)]
    pub struct Update {}
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// The name of the device.
    /// TODO: Decide if we should support device and chip name for accessories.
    pub name: String,
    /// Whether the device is visible in the UI.
    pub visible: bool,
    /// The position of the device in the simulated world.
    pub position: ProtoPosition,
    /// The orientation of the device.
    pub orientation: ProtoOrientation,
}

impl DeviceConfig {
    pub fn new(
        name: impl Into<String>,
        visible: bool,
        position: ProtoPosition,
        orientation: ProtoOrientation,
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
        respond_to: Responder<ListDeviceResponse>,
    },
    /// Update an existing device.
    Update {
        /// The patch to apply to the device.
        request: PatchDeviceRequest,
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
