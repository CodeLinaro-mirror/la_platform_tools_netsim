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

//! A BLE advertiser that sends legacy advertising PDUs.

use crate::{
    advertise_data::AdvertiseData,
    advertise_settings::AdvertiseSettings,
    error::Error,
    link_layer::{le_legacy_advertising_pdu::LeLegacyAdvertisingPdu, types::Address},
};
use std::time::Duration;

/// A BLE advertiser, containing its configuration and pre-built packets.
#[derive(Clone, Debug, PartialEq)]
pub struct Advertiser {
    /// The settings that control the advertiser's behavior.
    pub settings: AdvertiseSettings,
    /// The data payload for the advertising packet.
    pub data: AdvertiseData,
    /// The data payload for the scan response packet, if any.
    pub scan_response: Option<AdvertiseData>,
    /// The MAC address of the advertiser.
    pub mac_address: Address,
    advertising_packet: Vec<u8>,
    scan_response_packet: Option<Vec<u8>>,
}

impl Advertiser {
    /// Creates a new advertiser with the provided settings, data, and MAC address.
    ///
    /// This will construct the advertising and scan response packets and store
    /// them for later use. Returns an error if the provided data is too long
    /// to fit in a legacy advertising PDU.
    pub fn new(
        settings: AdvertiseSettings,
        data: AdvertiseData,
        scan_response: Option<AdvertiseData>,
        mac_address: Address,
    ) -> Result<Self, Error> {
        let packet_type = settings.get_packet_type();
        let data_bytes = data.as_bytes()?;
        if data_bytes.len() > 31 {
            return Err(Error::DataTooLong);
        }
        let advertising_packet = LeLegacyAdvertisingPdu::new(packet_type, mac_address, &data_bytes)
            .map_err(|_| Error::DataTooLong)?
            .as_bytes()
            .to_vec();

        let scan_response_packet = if let Some(sr_data) = &scan_response {
            let sr_data_bytes = sr_data.as_bytes()?;
            if sr_data_bytes.len() > 31 {
                return Err(Error::DataTooLong);
            }
            let pdu = LeLegacyAdvertisingPdu::new(
                crate::link_layer::types::LegacyAdvertisingType::ScanRsp,
                mac_address,
                &sr_data_bytes,
            )
            .map_err(|_| Error::DataTooLong)?;
            Some(pdu.as_bytes().to_vec())
        } else {
            None
        };

        Ok(Self {
            settings,
            data,
            scan_response,
            mac_address,
            advertising_packet,
            scan_response_packet,
        })
    }

    /// Returns the pre-built, constant advertising packet as a byte slice.
    pub fn advertising_packet(&self) -> &[u8] {
        &self.advertising_packet
    }

    /// Returns the pre-built, constant scan response packet as a byte slice,
    /// if one was provided.
    pub fn scan_response_packet(&self) -> Option<&[u8]> {
        self.scan_response_packet.as_deref()
    }

    /// Returns the wait time before sending the next packet.
    pub fn duration(&self) -> Duration {
        self.settings.mode.interval
    }

    /// Returns the total advertising duration, if one was set.
    pub fn timeout(&self) -> Option<Duration> {
        self.settings.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advertise_settings::{AdvertiseMode, AdvertiseSettingsBuilder, TxPowerLevel};

    #[test]
    fn test_advertiser_creation_and_getters() {
        let settings = AdvertiseSettingsBuilder::new()
            .mode(AdvertiseMode::new(Duration::from_millis(200)))
            .build();
        let data = AdvertiseData::builder("test-device".to_string(), TxPowerLevel::default())
            .include_device_name()
            .build()
            .unwrap();
        let address = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];

        let advertiser = Advertiser::new(settings, data.clone(), None, address).unwrap();

        assert_eq!(advertiser.duration(), Duration::from_millis(200));
        assert_eq!(advertiser.timeout(), None);
        assert!(advertiser.scan_response_packet().is_none());
        assert_eq!(advertiser.data.device_name, "test-device".to_string());
        assert_eq!(advertiser.mac_address, address);

        // Manually build the expected packet to compare
        let expected_data =
            [0x0c, 0x09, b't', b'e', b's', b't', b'-', b'd', b'e', b'v', b'i', b'c', b'e'];
        let expected_pdu_header = [0x02, 0x13]; // ADV_NONCONN_IND, length
        let mut expected_packet = Vec::<u8>::new();
        expected_packet.extend_from_slice(&expected_pdu_header);
        expected_packet.extend_from_slice(&address);
        expected_packet.extend_from_slice(&expected_data);

        assert_eq!(advertiser.advertising_packet(), expected_packet.as_slice());
    }
}
