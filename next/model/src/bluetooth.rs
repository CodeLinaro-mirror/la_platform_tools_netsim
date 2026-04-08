// Rust definitions for Bluetooth related structures
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::chip::{Radio, RadioUpdate};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Bluetooth {
    pub low_energy: Radio,
    pub classic: Radio,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BluetoothUpdate {
    pub classic: RadioUpdate,
    pub low_energy: RadioUpdate,
}

/// Parameters for creating a Bluetooth chip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothCreate {
    /// The Bluetooth address of the device.
    pub address: String,
    /// Rootcanal controller properties.
    pub bt_properties: Controller,
    /// The operational mode of the Bluetooth chip.
    pub mode: BluetoothMode,
}

/// An enum to differentiate between the kinds of Bluetooth chips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BluetoothMode {
    /// A full, virtual Bluetooth controller that can be paired with.
    Device(DeviceParams),
    /// A simple, non-interactive BLE beacon that broadcasts advertisements.
    Beacon(Box<BeaconParams>),
    /// A passive Bluetooth scanner to capture nearby traffic.
    Scanner(ScannerParams),
    /// A raw link-layer sniffer for baseband capture.
    Sniffer(SnifferParams),
}

/// Parameters for creating a virtual Bluetooth device.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceParams {}

/// Parameters for creating a BLE beacon.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BeaconParams {
    /// The BLE beacon's configuration.
    pub ble_beacon: beacon::BleBeacon,
}

/// Parameters for a Bluetooth scanner.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannerParams {
    pub active: bool,
}

/// Parameters for a Bluetooth sniffer.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnifferParams {
    // Future sniffer-specific properties can be added here.
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub vendor: String,
    pub product: String,
    pub version: String,
    pub address: String,
    pub properties: HashMap<String, String>,
}

impl Default for Controller {
    fn default() -> Self {
        Controller {
            vendor: "Netsim".to_string(),
            product: "Netsim".to_string(),
            version: "1.0".to_string(),
            address: "00:00:00:00:00:00".to_string(),
            properties: HashMap::new(),
        }
    }
}

pub mod beacon {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct BleBeacon {
        pub address: String,
        // Settings on how beacon functions
        pub settings: Option<AdvertiseSettings>,
        // Advertising Data
        pub adv_data: Option<AdvertiseData>,
        // Scan Response Data
        pub scan_response: Option<AdvertiseData>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct AdvertiseSettings {
        pub interval: Option<Interval>,
        pub tx_power: Option<TxPower>,
        pub scannable: bool,
        pub timeout: u64,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum Interval {
        AdvertiseMode(AdvertiseMode),
        Milliseconds(u64),
    }

    impl Default for Interval {
        fn default() -> Self {
            Interval::AdvertiseMode(AdvertiseMode::LowPower)
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum TxPower {
        TxPowerLevel(AdvertiseTxPower),
        Dbm(i32),
    }

    impl Default for TxPower {
        fn default() -> Self {
            TxPower::TxPowerLevel(AdvertiseTxPower::UltraLow)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    pub enum AdvertiseMode {
        #[default]
        LowPower = 0,
        Balanced = 1,
        LowLatency = 2,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    pub enum AdvertiseTxPower {
        #[default]
        UltraLow = 0,
        Low = 1,
        Medium = 2,
        High = 3,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct AdvertiseData {
        pub include_device_name: bool,
        pub include_tx_power_level: bool,
        pub manufacturer_data: Vec<u8>,
        pub services: Vec<Service>,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    pub struct Service {
        pub uuid: String,
        pub data: Vec<u8>,
    }
}
