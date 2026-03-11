use std::fmt;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::{
    chip::{ChipConfig, ChipId, PacketSink, PacketStream},
    client_error::ClientError,
    client_method,
    device_error::DeviceError,
};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Clone, Debug)]
pub struct DeviceClient {
    sender: mpsc::Sender<DeviceRequest>,
}

impl DeviceClient {
    pub fn new(sender: mpsc::Sender<DeviceRequest>) -> Self {
        Self { sender }
    }

    pub async fn shutdown(&self) -> Result<(), ClientError> {
        self.sender
            .send(DeviceRequest::Shutdown)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        Ok(())
    }

    pub fn notify_chip_removed(&self, device_id: DeviceId, chip_id: ChipId) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender
                .send(DeviceRequest::NotifyChipRemoved { device_id, chip_id, respond_to: None })
                .await
            {
                log::error!("Failed to send NotifyChipRemoved for chip {chip_id}: {e}");
            }
        });
    }

    pub async fn notify_chip_removed_block(
        &self,
        device_id: DeviceId,
        chip_id: ChipId,
    ) -> Result<(), ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceRequest::NotifyChipRemoved { device_id, chip_id, respond_to: Some(tx) })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?;
        Ok(())
    }
}

client_method!(DeviceClient => fn create(request: Box<api::DeviceCreate>) -> DeviceId as DeviceRequest::Create);
client_method!(DeviceClient => fn add_chip(request: DeviceAddChip) -> () as DeviceRequest::AddChip);
client_method!(DeviceClient => fn list() -> api::ListDeviceResponse as DeviceRequest::List);
client_method!(DeviceClient => fn update(update: api::DeviceUpdate) -> () as DeviceRequest::Update);
client_method!(DeviceClient => fn delete(id: DeviceId) -> () as DeviceRequest::Delete);
client_method!(DeviceClient => fn reset() -> () as DeviceRequest::Reset);

pub mod api {
    use serde::{Deserialize, Serialize};

    use crate::{
        chip::{BleBeacon, BluetoothCreate, CellCreate, ChipConfig, UwbCreate, WifiCreate},
        device::{Device, DeviceConfig, Orientation, Position},
    };

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct ListDeviceResponse {
        pub devices: Vec<Device>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct DeviceUpdate {
        pub id: u32,
        pub name: Option<String>,
        pub visible: Option<bool>,
        //TODO: pub chip_id: Option<ChipId>,
        pub position: Option<Position>,
        pub orientation: Option<Orientation>,
        //TODO: pub links: Option<Vec<Link>,
        pub chips: Option<Vec<crate::chip::ChipUpdate>>,
    }

    #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
    pub struct DeviceCreate {
        pub device_config: DeviceConfig,
        pub chip: DeviceChipCreate,
    }

    #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
    pub struct DeviceChipCreate {
        pub name: String,
        pub manufacturer: String,
        pub product_name: String,
        pub chip: Chip,
    }

    impl DeviceChipCreate {
        pub fn new(
            name: impl Into<String>,
            manufacturer: impl Into<String>,
            product_name: impl Into<String>,
            chip: Chip,
        ) -> DeviceChipCreate {
            DeviceChipCreate {
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
        Bluetooth(BluetoothCreate),
        Wifi(WifiCreate),
        Uwb(UwbCreate),
        Cell(CellCreate),
    }

    impl Default for Chip {
        fn default() -> Self {
            Chip::Beacon(BleBeacon::default())
        }
    }

    impl From<crate::chip::ChipKindParams> for Chip {
        fn from(params: crate::chip::ChipKindParams) -> Self {
            match params {
                crate::chip::ChipKindParams::Bluetooth(bt) => match bt.mode {
                    crate::chip::BluetoothMode::Beacon(beacon_params) => {
                        Chip::Beacon(beacon_params.ble_beacon)
                    }
                    _ => Chip::Bluetooth(bt),
                },
                crate::chip::ChipKindParams::Wifi(wifi) => Chip::Wifi(wifi),
                crate::chip::ChipKindParams::Uwb(uwb) => Chip::Uwb(uwb),
                crate::chip::ChipKindParams::Cell(cell) => Chip::Cell(cell),
            }
        }
    }

    impl From<Chip> for crate::chip::ChipKindParams {
        fn from(chip: Chip) -> Self {
            match chip {
                Chip::Beacon(beacon) => {
                    crate::chip::ChipKindParams::Bluetooth(crate::chip::BluetoothCreate {
                        address: beacon.address.clone(),
                        bt_properties: Default::default(),
                        mode: crate::chip::BluetoothMode::Beacon(Box::new(
                            crate::chip::BeaconParams { ble_beacon: beacon },
                        )),
                    })
                }
                Chip::Bluetooth(bt) => crate::chip::ChipKindParams::Bluetooth(bt),
                Chip::Wifi(wifi) => crate::chip::ChipKindParams::Wifi(wifi),
                Chip::Uwb(uwb) => crate::chip::ChipKindParams::Uwb(uwb),
                Chip::Cell(cell) => crate::chip::ChipKindParams::Cell(cell),
            }
        }
    }

    impl From<DeviceChipCreate> for ChipConfig {
        fn from(create: DeviceChipCreate) -> Self {
            ChipConfig {
                name: create.name,
                manufacturer: create.manufacturer,
                product_name: create.product_name,
                chip_kind_params: create.chip.into(),
            }
        }
    }

    impl From<ChipConfig> for DeviceChipCreate {
        fn from(config: ChipConfig) -> Self {
            DeviceChipCreate {
                name: config.name,
                manufacturer: config.manufacturer,
                product_name: config.product_name,
                chip: config.chip_kind_params.into(),
            }
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
    pub builtin: bool,
    pub chips: Vec<crate::chip::Chip>,
    pub device_info: Option<DeviceInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub kind: String,
    pub version: String,
    pub sdk_version: String,
    pub build_id: String,
    pub variant: String,
    pub arch: String,
    pub avd_path: String,
}

impl From<netsim_types::DeviceInfo> for DeviceInfo {
    fn from(info: netsim_types::DeviceInfo) -> Self {
        Self {
            name: info.name,
            kind: info.kind,
            version: info.version,
            sdk_version: info.sdk_version,
            build_id: info.build_id,
            variant: info.variant,
            arch: info.arch,
            avd_path: info.avd_path,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub visible: bool,
    pub position: Position,
    pub orientation: Orientation,
    pub builtin: bool,
    pub device_info: Option<DeviceInfo>,
}

impl DeviceConfig {
    pub fn new(
        name: impl Into<String>,
        visible: bool,
        position: Position,
        orientation: Orientation,
        builtin: bool,
    ) -> DeviceConfig {
        DeviceConfig {
            name: name.into(),
            visible,
            position,
            orientation,
            builtin,
            device_info: None,
        }
    }
}

pub struct DeviceAddChip {
    pub device_guid: String,
    pub packet_stream: Option<PacketStream>,
    pub packet_sink: Option<PacketSink>,
    pub device_config: DeviceConfig,
    pub chip_config: ChipConfig,
}
impl fmt::Debug for DeviceAddChip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceAddChip")
            .field("device_guid", &self.device_guid)
            .field("device_config", &self.device_config)
            .field("chip_config", &self.chip_config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum DeviceRequest {
    AddChip {
        request: DeviceAddChip,
        respond_to: Responder<()>,
    },
    Create {
        request: Box<api::DeviceCreate>,
        respond_to: Responder<DeviceId>,
    },
    List {
        respond_to: Responder<api::ListDeviceResponse>,
    },
    Update {
        update: api::DeviceUpdate,
        respond_to: Responder<()>,
    },
    Delete {
        id: DeviceId,
        respond_to: Responder<()>,
    },
    Reset {
        respond_to: Responder<()>,
    },
    GetChipStatistics {
        respond_to: Responder<()>,
    },
    NotifyChipRemoved {
        device_id: DeviceId,
        chip_id: ChipId,
        respond_to: Option<oneshot::Sender<()>>,
    },
    Shutdown,
}
