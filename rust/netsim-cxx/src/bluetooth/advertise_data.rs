// Copyright 2023 Google LLC
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

//! Builder for Advertising Data

use frontend_proto::model::{
    chip::bluetooth_beacon::AdvertiseData as AdvertiseDataProto,
    chip_create::BluetoothBeaconCreate as BluetoothBeaconCreateProto,
};
use std::convert::TryInto;

// Core Specification (v5.3 Vol 6 Part B §2.3.1.3 and §2.3.1.4)
const MAX_ADV_NONCONN_DATA_LEN: usize = 31;

// Assigned Numbers Document (§2.3)
const AD_TYPE_COMPLETE_NAME: u8 = 0x09;
const AD_TYPE_TX_POWER: u8 = 0x0A;
const AD_TYPE_MANUFACTURER_DATA: u8 = 0xFF;

#[derive(Default)]
/// Builder for the advertisement data field of a Bluetooth packet.
pub struct Builder {
    device_name: Option<String>,
    tx_power: Option<i8>,
    manufacturer_data: Option<Vec<u8>>,
}

pub struct AdvertiseData {
    /// Raw bytes to be sent in the advertise data field of a BLE advertisement packet.
    pub bytes: Vec<u8>,
    /// Protobuf representation of the advertise data.
    pub proto: AdvertiseDataProto,
}

impl Builder {
    /// Returns a new advertisement data builder with no fields.
    pub fn new() -> Self {
        Builder::default()
    }

    pub fn from_proto(device_name: String, tx_power_level: i8, proto: &AdvertiseDataProto) -> Self {
        let mut builder = Self::new();

        if proto.include_device_name {
            builder.device_name(device_name);
        }

        if proto.include_tx_power_level {
            builder.tx_power(tx_power_level);
        }

        if !proto.manufacturer_data.is_empty() {
            builder.manufacturer_data(proto.manufacturer_data.clone());
        }

        builder
    }

    /// Build the advertisement data.
    ///
    /// Returns a vector of bytes holding the serialized advertisement data based on the fields added to the builder, or `Err(String)` if the data would be malformed.
    pub fn build(&mut self) -> Result<AdvertiseData, String> {
        let mut bytes = Vec::new();
        let mut proto = AdvertiseDataProto::new();

        if let Some(device_name) = &self.device_name {
            let device_name = device_name.as_bytes();

            if device_name.len() > MAX_ADV_NONCONN_DATA_LEN - 2 {
                return Err(format!(
                    "complete name must be less than {} chars",
                    MAX_ADV_NONCONN_DATA_LEN - 2
                ));
            }

            bytes.extend(vec![
                (1 + device_name.len())
                    .try_into()
                    .map_err(|_| "complete name must be less than 255 chars")?,
                AD_TYPE_COMPLETE_NAME,
            ]);
            bytes.extend_from_slice(device_name);
            proto.include_device_name = true;
        }

        if let Some(tx_power) = self.tx_power {
            bytes.extend(vec![2, AD_TYPE_TX_POWER, tx_power as u8]);
            proto.include_tx_power_level = true;
        }

        if let Some(manufacturer_data) = &self.manufacturer_data {
            if manufacturer_data.len() < 2 {
                // Supplement to the Core Specification (v10 Part A §1.4.2)
                return Err("manufacturer data must be at least 2 bytes".to_string());
            }

            if manufacturer_data.len() > MAX_ADV_NONCONN_DATA_LEN - 2 {
                return Err(format!(
                    "manufacturer data must be less than {} bytes",
                    MAX_ADV_NONCONN_DATA_LEN - 2
                ));
            }

            bytes.extend(vec![
                (1 + manufacturer_data.len())
                    .try_into()
                    .map_err(|_| "manufacturer data must be less than 255 bytes")?,
                AD_TYPE_MANUFACTURER_DATA,
            ]);
            bytes.extend_from_slice(manufacturer_data);
            proto.manufacturer_data = manufacturer_data.clone();
        }

        if bytes.len() > MAX_ADV_NONCONN_DATA_LEN {
            return Err(format!(
                "exceeded maximum advertising packet length of {} bytes",
                MAX_ADV_NONCONN_DATA_LEN
            ));
        }

        Ok(AdvertiseData { bytes, proto })
    }

    /// Add a complete device name field to the advertisement data.
    pub fn device_name(&mut self, device_name: String) -> &mut Self {
        self.device_name = Some(device_name);
        self
    }

    /// Add a transmit power field to the advertisement data.
    pub fn tx_power(&mut self, tx_power: i8) -> &mut Self {
        // TODO(jmes) support constant-named tx_power_levels as defined in
        // https://developer.android.com/reference/android/bluetooth/le/AdvertiseSettings.Builder#setTxPowerLevel(int)
        self.tx_power = Some(tx_power);
        self
    }

    /// Add a manufacturer data field to the advertisement data.
    pub fn manufacturer_data(&mut self, manufacturer_data: Vec<u8>) -> &mut Self {
        self.manufacturer_data = Some(manufacturer_data);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend_proto::model::chip::bluetooth_beacon::AdvertiseSettings as AdvertiseSettingsProto;
    use protobuf::MessageField;

    const HEADER_LEN: usize = 2;

    #[test]
    fn test_from_proto_succeeds() {
        let device_name = String::from("test-device-name");
        let tx_power: i8 = 1;
        let exp_name_len = HEADER_LEN + device_name.len();
        let exp_tx_power_len = HEADER_LEN + 1;

        let ad = Builder::from_proto(
            device_name.clone(),
            tx_power,
            &AdvertiseDataProto {
                include_device_name: true,
                include_tx_power_level: true,
                ..Default::default()
            },
        )
        .build();

        assert!(ad.is_ok());
        let bytes = ad.unwrap().bytes;

        assert_eq!(exp_name_len + exp_tx_power_len, bytes.len());
        assert_eq!(
            [
                vec![(exp_name_len - 1) as u8, AD_TYPE_COMPLETE_NAME],
                device_name.into_bytes(),
                vec![(exp_tx_power_len - 1) as u8, AD_TYPE_TX_POWER, tx_power as u8]
            ]
            .concat(),
            bytes
        );
    }

    #[test]
    fn test_from_proto_fails() {
        let device_name = "a".repeat(MAX_ADV_NONCONN_DATA_LEN - HEADER_LEN + 1);
        let data = Builder::from_proto(
            device_name,
            0,
            &AdvertiseDataProto { include_device_name: true, ..Default::default() },
        )
        .build();

        assert!(data.is_err());
    }

    #[test]
    fn test_from_proto_sets_proto_field() {
        let device_name = String::from("test-device-name");
        let tx_power: i8 = 1;
        let ad_proto = AdvertiseDataProto {
            include_device_name: true,
            include_tx_power_level: true,
            ..Default::default()
        };

        let ad = Builder::from_proto(device_name.clone(), tx_power, &ad_proto).build();

        assert!(ad.is_ok());
        assert_eq!(ad_proto, ad.unwrap().proto);
    }

    #[test]
    fn test_set_device_name_succeeds() {
        let device_name = String::from("test-device-name");
        let ad = Builder::new().device_name(device_name.clone()).build();
        let exp_len = HEADER_LEN + device_name.len();

        assert!(ad.is_ok());
        let bytes = ad.unwrap().bytes;

        assert_eq!(exp_len, bytes.len());
        assert_eq!(
            [vec![(exp_len - 1) as u8, AD_TYPE_COMPLETE_NAME], device_name.into_bytes()].concat(),
            bytes
        );
    }

    #[test]
    fn test_set_device_name_fails() {
        let device_name = "a".repeat(MAX_ADV_NONCONN_DATA_LEN - HEADER_LEN + 1);
        let data = Builder::new().device_name(device_name).build();

        assert!(data.is_err());
    }

    #[test]
    fn test_set_tx_power() {
        let tx_power: i8 = -6;
        let ad = Builder::new().tx_power(tx_power).build();
        let exp_len = HEADER_LEN + 1;

        assert!(ad.is_ok());
        let bytes = ad.unwrap().bytes;

        assert_eq!(exp_len, bytes.len());
        assert_eq!(vec![(exp_len - 1) as u8, AD_TYPE_TX_POWER, tx_power as u8], bytes);
    }

    #[test]
    fn test_set_manufacturer_data_succeeds() {
        let manufacturer_data = String::from("test-manufacturer-data");
        let ad = Builder::new().manufacturer_data(manufacturer_data.clone().into_bytes()).build();
        let exp_len = HEADER_LEN + manufacturer_data.len();

        assert!(ad.is_ok());
        let bytes = ad.unwrap().bytes;

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
        let data = Builder::new().manufacturer_data(manufacturer_data.into_bytes()).build();

        assert!(data.is_err());
    }

    #[test]
    fn test_set_name_and_power_succeeds() {
        let exp_data = [
            0x0F, 0x09, b'g', b'D', b'e', b'v', b'i', b'c', b'e', b'-', b'b', b'e', b'a', b'c',
            b'o', b'n', 0x02, 0x0A, 0x0,
        ];
        let data = Builder::new().device_name(String::from("gDevice-beacon")).tx_power(0).build();

        assert!(data.is_ok());
        assert_eq!(exp_data, data.unwrap().bytes.as_slice());
    }
}
