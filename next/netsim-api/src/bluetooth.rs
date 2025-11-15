// Rust definitions for Bluetooth related structures
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Bluetooth {
    pub low_energy: Option<Box<super::chips::Radio>>,
    pub classic: Option<Box<super::chips::Radio>>,
    pub address: String,
    pub bt_properties: Option<Controller>,
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
        pub bt: Option<super::Bluetooth>,
        pub address: String,
        pub settings: Option<AdvertiseSettings>,
        pub adv_data: Option<AdvertiseData>,
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
