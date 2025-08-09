// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Builder for the data payload of a BLE advertising or scan response packet.

use super::advertise_settings::TxPowerLevel;
use crate::error::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned};

// Core Specification (v5.3 Vol 6 Part B §2.3.1.3 and §2.3.1.4)
const MAX_ADV_NONCONN_DATA_LEN: usize = 31;

// Assigned Numbers Document (§2.3)
const AD_TYPE_COMPLETE_NAME: u8 = 0x09;
const AD_TYPE_TX_POWER: u8 = 0x0A;
const AD_TYPE_MANUFACTURER_DATA: u8 = 0xFF;
const AD_TYPE_SERVICE_DATA_16_BIT_UUID: u8 = 0x16;

#[derive(Debug, FromBytes, IntoBytes, Unaligned, Immutable)]
#[repr(packed)]
#[allow(dead_code)]
struct AdStructureHeader {
    len: u8,
    ad_type: u8,
}

/// A GATT service to be advertised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// The 16-bit UUID of the service, represented as a hex string (e.g., "180D").
    pub uuid: String,
    /// The data associated with the service.
    pub data: Vec<u8>,
}

/// The data payload for an advertising or scan response packet.
///
/// Use the `builder()` to construct this struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvertiseData {
    /// Whether or not to include the device name in the packet.
    pub include_device_name: bool,
    /// Whether or not to include the transmit power in the packet.
    pub include_tx_power_level: bool,
    /// Manufacturer-specific data.
    pub manufacturer_data: Option<Vec<u8>>,
    /// GATT services.
    pub services: Vec<Service>,
    /// The device name.
    pub device_name: String,
    tx_power_level: TxPowerLevel,
}

impl AdvertiseData {
    /// Returns a new builder for creating `AdvertiseData`.
    pub fn builder(device_name: String, tx_power_level: TxPowerLevel) -> AdvertiseDataBuilder {
        AdvertiseDataBuilder::new(device_name, tx_power_level)
    }

    /// Gets the raw bytes of the serialized advertising data structures.
    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();

        if self.include_device_name {
            let device_name = self.device_name.as_bytes();
            let len = 1 + device_name.len();

            if len > MAX_ADV_NONCONN_DATA_LEN - 2 {
                return Err(Error::DataTooLong);
            }

            let header = AdStructureHeader { len: len as u8, ad_type: AD_TYPE_COMPLETE_NAME };
            bytes.extend_from_slice(header.as_bytes());
            bytes.extend_from_slice(device_name);
        }

        if self.include_tx_power_level {
            let header = AdStructureHeader { len: 2, ad_type: AD_TYPE_TX_POWER };
            bytes.extend_from_slice(header.as_bytes());
            bytes.push(self.tx_power_level.dbm as u8);
        }

        if let Some(manufacturer_data) = &self.manufacturer_data {
            if manufacturer_data.len() < 2 {
                // Supplement to the Core Specification (v10 Part A §1.4.2)
                return Err(Error::InvalidInput(
                    "manufacturer data must be at least 2 bytes".to_string(),
                ));
            }

            let len = 1 + manufacturer_data.len();
            if len > MAX_ADV_NONCONN_DATA_LEN - 2 {
                return Err(Error::DataTooLong);
            }

            let header = AdStructureHeader { len: len as u8, ad_type: AD_TYPE_MANUFACTURER_DATA };
            bytes.extend_from_slice(header.as_bytes());
            bytes.extend_from_slice(manufacturer_data);
        }

        for service in &self.services {
            // For now, only support 16-bit UUIDs as per the protobuf example.
            let uuid_bytes = hex::decode(&service.uuid)
                .map_err(|e| Error::InvalidInput(format!("Invalid service UUID: {}", e)))?;
            if uuid_bytes.len() != 2 {
                return Err(Error::InvalidInput(
                    "Only 16-bit service UUIDs are currently supported".to_string(),
                ));
            }
            let len = 1 + uuid_bytes.len() + service.data.len();
            let header =
                AdStructureHeader { len: len as u8, ad_type: AD_TYPE_SERVICE_DATA_16_BIT_UUID };
            bytes.extend_from_slice(header.as_bytes());
            bytes.extend_from_slice(&uuid_bytes);
            bytes.extend_from_slice(&service.data);
        }

        if bytes.len() > MAX_ADV_NONCONN_DATA_LEN {
            return Err(Error::DataTooLong);
        }

        Ok(bytes)
    }
}

/// A builder for creating an `AdvertiseData` payload.
#[derive(Default)]
pub struct AdvertiseDataBuilder {
    device_name: String,
    tx_power_level: TxPowerLevel,
    include_device_name: bool,
    include_tx_power_level: bool,
    manufacturer_data: Option<Vec<u8>>,
    services: Vec<Service>,
}

impl AdvertiseDataBuilder {
    /// Returns a new advertise data builder with empty fields.
    pub fn new(device_name: String, tx_power_level: TxPowerLevel) -> Self {
        AdvertiseDataBuilder { device_name, tx_power_level, ..Self::default() }
    }

    /// Build the `AdvertiseData`.
    ///
    /// Returns an error if the resulting data would be malformed or exceed the
    /// maximum allowed length for a legacy advertising packet.
    pub fn build(&self) -> Result<AdvertiseData, Error> {
        let ad = AdvertiseData {
            include_device_name: self.include_device_name,
            include_tx_power_level: self.include_tx_power_level,
            manufacturer_data: self.manufacturer_data.clone(),
            services: self.services.clone(),
            device_name: self.device_name.clone(),
            tx_power_level: self.tx_power_level,
        };
        // pre-validate the length
        ad.as_bytes()?;
        Ok(ad)
    }

    /// Includes the device name in the advertising data.
    pub fn include_device_name(&mut self) -> &mut Self {
        self.include_device_name = true;
        self
    }

    /// Includes the transmit power level in the advertising data.
    pub fn include_tx_power_level(&mut self) -> &mut Self {
        self.include_tx_power_level = true;
        self
    }

    /// Sets the manufacturer-specific data.
    pub fn manufacturer_data(&mut self, manufacturer_data: Vec<u8>) -> &mut Self {
        self.manufacturer_data = Some(manufacturer_data);
        self
    }

    /// Sets the list of GATT services to advertise.
    pub fn services(&mut self, services: Vec<Service>) -> &mut Self {
        self.services = services;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_LEN: usize = 2;

    #[test]
    fn test_set_device_name_succeeds() {
        let device_name = String::from("test-device-name");
        let ad = AdvertiseData::builder(device_name.clone(), TxPowerLevel::default())
            .include_device_name()
            .build();
        let exp_len = HEADER_LEN + device_name.len();

        assert!(ad.is_ok());
        let bytes = ad.unwrap().as_bytes().unwrap();

        assert_eq!(exp_len, bytes.len());
        assert_eq!(
            [vec![(exp_len - 1) as u8, AD_TYPE_COMPLETE_NAME], device_name.into_bytes()].concat(),
            bytes
        );
    }

    #[test]
    fn test_set_device_name_fails() {
        let device_name = "a".repeat(MAX_ADV_NONCONN_DATA_LEN - HEADER_LEN + 1);
        let data = AdvertiseData::builder(device_name, TxPowerLevel::default())
            .include_device_name()
            .build();

        assert!(matches!(data, Err(Error::DataTooLong)));
    }

    #[test]
    fn test_set_tx_power_level() {
        let tx_power = TxPowerLevel::new(-6);
        let ad =
            AdvertiseData::builder(String::default(), tx_power).include_tx_power_level().build();
        let exp_len = HEADER_LEN + 1;

        assert!(ad.is_ok());
        let bytes = ad.unwrap().as_bytes().unwrap();

        assert_eq!(exp_len, bytes.len());
        assert_eq!(vec![(exp_len - 1) as u8, AD_TYPE_TX_POWER, tx_power.dbm as u8], bytes);
    }

    #[test]
    fn test_set_manufacturer_data_succeeds() {
        let manufacturer_data = String::from("test-manufacturer-data");
        let ad = AdvertiseData::builder(String::default(), TxPowerLevel::default())
            .manufacturer_data(manufacturer_data.clone().into_bytes())
            .build();
        let exp_len = HEADER_LEN + manufacturer_data.len();

        assert!(ad.is_ok());
        let bytes = ad.unwrap().as_bytes().unwrap();

        assert_eq!(exp_len, bytes.len());
        assert_eq!(
            [vec![(exp_len - 1) as u8, AD_TYPE_MANUFACTURER_DATA], manufacturer_data.into_bytes()]
                .concat(),
            bytes
        );
    }

    #[test]
    fn test_set_manufacturer_data_fails() {
        let manufacturer_data = "a".repeat(MAX_ADV_NONCONN_DATA_LEN - HEADER_LEN + 1);
        let data = AdvertiseData::builder(String::default(), TxPowerLevel::default())
            .manufacturer_data(manufacturer_data.into_bytes())
            .build();

        assert!(matches!(data, Err(Error::DataTooLong)));
    }

    #[test]
    fn test_set_name_and_power_succeeds() {
        let exp_data = [
            0x0F, 0x09, b'g', b'D', b'e', b'v', b'i', b'c', b'e', b'-', b'b', b'e', b'a', b'c',
            b'o', b'n', 0x02, 0x0A, 0x0,
        ];
        let data = AdvertiseData::builder(String::from("gDevice-beacon"), TxPowerLevel::new(0))
            .include_device_name()
            .include_tx_power_level()
            .build();

        assert!(data.is_ok());
        assert_eq!(exp_data, data.unwrap().as_bytes().unwrap().as_slice());
    }

    #[test]
    fn test_set_services_succeeds() {
        let services = vec![Service { uuid: "180D".to_string(), data: vec![0x01, 0x02] }];
        let ad = AdvertiseData::builder(String::default(), TxPowerLevel::default())
            .services(services)
            .build();
        assert!(ad.is_ok());
        let bytes = ad.unwrap().as_bytes().unwrap();
        // len(1) + type(1) + uuid(2) + data(2) = 6
        // AD len = 1 + uuid + data = 5
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes, vec![0x05, 0x16, 0x18, 0x0D, 0x01, 0x02]);
    }

    #[test]
    fn test_set_services_invalid_uuid() {
        let services = vec![Service { uuid: "invalid".to_string(), data: vec![] }];
        let ad = AdvertiseData::builder(String::default(), TxPowerLevel::default())
            .services(services)
            .build();
        assert!(matches!(ad, Err(Error::InvalidInput(_))));
    }

    #[test]
    fn test_build_data_too_long() {
        let data = AdvertiseData::builder("a".repeat(40).to_string(), TxPowerLevel::default())
            .include_device_name()
            .build();
        assert!(matches!(data, Err(Error::DataTooLong)));
    }
}
