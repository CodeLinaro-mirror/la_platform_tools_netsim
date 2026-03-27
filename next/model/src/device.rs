use std::fmt;

use serde::{Deserialize, Serialize};

use crate::chip::{ChipConfig, PacketSink, PacketStream};

// DEVICE SERVICE
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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

pub mod api {
    use serde::{Deserialize, Serialize};

    use crate::{
        chip::{
            ApCreate, BleBeacon, BluetoothCreate, CellCreate, ChipConfig, UwbCreate, WifiCreate,
            WifiMode,
        },
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

    impl DeviceCreate {
        pub fn default_ap(ssid_override: Option<String>) -> Self {
            let name = ssid_override.unwrap_or_else(|| crate::ap::DEFAULT_WIFI_SSID.to_string());

            let ap_create = ApCreate {
                ssid: name.clone(),
                bssid: "02:00:00:44:55:66".to_string(),
                channel: 6,
                hw_mode: WifiMode::G,
                wpa_passphrase: None,
                beacon_interval: 100,
                country_code: None,
                dtim_period: 2,
                hidden_ssid: false,
                sae: false,
                wmm_enabled: true,
                enterprise_enabled: false,
                mac_acl_mode: 0,
                mac_acl_list: vec![],
                ftm_responder_enabled: true,
            };

            Self {
                device_config: DeviceConfig {
                    name,
                    position: Default::default(),
                    orientation: Default::default(),
                    visible: false,
                    builtin: true,
                    device_info: None,
                },
                chip: DeviceChipCreate {
                    name: "main-ap".to_string(),
                    manufacturer: "Google".to_string(),
                    product_name: "AccessPoint".to_string(),
                    chip: Chip::Ap(ap_create),
                },
            }
        }
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
        Ap(ApCreate),
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
                crate::chip::ChipKindParams::Ap(ap) => Chip::Ap(ap),
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
                Chip::Ap(ap) => crate::chip::ChipKindParams::Ap(ap),
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
