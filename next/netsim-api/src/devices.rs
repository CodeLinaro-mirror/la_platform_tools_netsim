use crate::chips::{ChipConfig, ChipId, PacketSink, PacketStream};
use crate::client_error::ClientError;
use crate::client_method;
use crate::device_error::DeviceError;
use netsim_proto::frontend::ListDeviceResponse;
use netsim_proto::frontend::PatchDeviceRequest;
use netsim_proto::model::Device;
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
}

// Generate client methods.
client_method!(DeviceClient => fn ps_create(params: CreateDeviceParams) -> () as DeviceRequest::PsCreate);
client_method!(DeviceClient => fn list() -> ListDeviceResponse as DeviceRequest::List);

#[allow(dead_code)]
pub struct GetVersionMessage {
    response: oneshot::Sender<String>,
}

pub mod api {
    use netsim_proto::model::chip::ble_beacon::{
        AdvertiseData as ProtoAdvertiseData, AdvertiseSettings as ProtoAdvertiseSettings,
    };
    use netsim_proto::model::{Orientation as ProtoOrientation, Position as ProtoPosition};

    // TODO: Revisit the APIs to separate the Api from the Domain.

    /// The top-level parameters for creating any kind of chip.
    #[derive(Debug)]
    pub struct DeviceCreate {
        pub name: String,
        pub position: ProtoPosition,
        pub orientation: ProtoOrientation,
        pub chips: Vec<ChipCreate>,
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

    #[derive(Debug)]
    pub enum Chip {
        Beacon(BleBeacon),
    }

    #[derive(Debug)]
    pub struct BleBeacon {
        // BD_ADDR address
        pub address: String,
        // Settings on how beacon functions
        pub settings: ProtoAdvertiseSettings,
        // Advertising Data
        pub adv_data: ProtoAdvertiseData,
        // Scan Response Data
        pub scan_response: ProtoAdvertiseData,
    }

    #[derive(Debug)]
    pub struct Update {}
}

impl From<api::BleBeacon> for netsim_proto::model::chip::BleBeacon {
    fn from(beacon: api::BleBeacon) -> Self {
        netsim_proto::model::chip::BleBeacon {
            address: beacon.address,
            settings: Some(beacon.settings).into(),
            adv_data: Some(beacon.adv_data).into(),
            scan_response: Some(beacon.scan_response).into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// The name of the device.
    pub name: String,
    /// Whether the device is visible in the UI.
    pub visible: bool,
    /// The position of the device in the simulated world.
    pub position: ProtoPosition,
    /// The orientation of the device.
    pub orientation: ProtoOrientation,
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
        device: Device,
        /// The channel to send the created device back on.
        respond_to: Responder<Device>,
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
        /// Chip Identifier
        id: ChipId,
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
