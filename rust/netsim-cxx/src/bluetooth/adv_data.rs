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

impl Builder {
    /// Returns a new advertisement data builder with no fields.
    pub fn new() -> Self {
        Builder::default()
    }

    /// Build the advertisement data.
    ///
    /// Returns a vector of bytes holding the serialized advertisement data based on the fields added to the builder, or `Err(String)` if the data would be malformed.
    pub fn build(&mut self) -> Result<Vec<u8>, String> {
        let mut data = Vec::new();

        if let Some(device_name) = &self.device_name {
            let device_name = device_name.as_bytes();

            if device_name.len() > MAX_ADV_NONCONN_DATA_LEN - 2 {
                return Err(format!(
                    "complete name must be less than {} chars",
                    MAX_ADV_NONCONN_DATA_LEN - 2
                ));
            }

            data.extend(vec![
                (1 + device_name.len())
                    .try_into()
                    .map_err(|_| "complete name must be less than 255 chars")?,
                AD_TYPE_COMPLETE_NAME,
            ]);
            data.extend_from_slice(device_name);
        }

        if let Some(tx_power) = self.tx_power {
            data.extend(vec![2, AD_TYPE_TX_POWER, tx_power as u8]);
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

            data.extend(vec![
                (1 + manufacturer_data.len())
                    .try_into()
                    .map_err(|_| "manufacturer data must be less than 255 bytes")?,
                AD_TYPE_MANUFACTURER_DATA,
            ]);
            data.extend_from_slice(manufacturer_data);
        }

        if data.len() > MAX_ADV_NONCONN_DATA_LEN {
            return Err(format!(
                "exceeded maximum advertising packet length of {} bytes",
                MAX_ADV_NONCONN_DATA_LEN
            ));
        }

        Ok(data)
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

    const HEADER_LEN: usize = 2;

    #[test]
    fn test_set_device_name_succeeds() {
        let device_name = String::from("test-device-name");
        let data = Builder::new().device_name(device_name.clone()).build();
        let exp_len = HEADER_LEN + device_name.len();

        assert!(data.is_ok());
        let data = data.unwrap();

        assert_eq!(exp_len, data.len());
        assert_eq!(
            [vec![(exp_len - 1) as u8, AD_TYPE_COMPLETE_NAME], device_name.into_bytes()].concat(),
            data
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
        let data = Builder::new().tx_power(tx_power).build();
        let exp_len = HEADER_LEN + 1;

        assert!(data.is_ok());
        let data = data.unwrap();

        assert_eq!(exp_len, data.len());
        assert_eq!(vec![(exp_len - 1) as u8, AD_TYPE_TX_POWER, tx_power as u8], data);
    }

    #[test]
    fn test_set_manufacturer_data_succeeds() {
        let manufacturer_data = String::from("test-manufacturer-data");
        let data = Builder::new().manufacturer_data(manufacturer_data.clone().into_bytes()).build();
        let exp_len = HEADER_LEN + manufacturer_data.len();

        assert!(data.is_ok());
        let data = data.unwrap();

        assert_eq!(exp_len, data.len());
        assert_eq!(
            [vec![(exp_len - 1) as u8, AD_TYPE_MANUFACTURER_DATA], manufacturer_data.into_bytes()]
                .concat(),
            data
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
        assert_eq!(exp_data, data.unwrap().as_slice());
    }
}
