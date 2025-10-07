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

use crate::link_layer::types::{Address, LegacyAdvertisingType};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const PDU_HEADER_LEN: u8 = 2;

#[derive(FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable)]
#[repr(C, packed)]
struct LeLegacyAdvertisingPduHeader {
    pdu_type: u8,
    length: u8,
}

/// A LE legacy advertising PDU.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LeLegacyAdvertisingPdu {
    inner: Vec<u8>,
}

impl LeLegacyAdvertisingPdu {
    /// Creates a new legacy advertising PDU.
    pub fn new(
        adv_type: LegacyAdvertisingType,
        adv_address: Address,
        adv_data: &[u8],
    ) -> Result<Self, String> {
        let body_len = adv_address.len() + adv_data.len();
        let pdu_len = PDU_HEADER_LEN as usize + body_len;
        let mut inner = vec![0; pdu_len];

        let (header_bytes, body_bytes) = inner.split_at_mut(PDU_HEADER_LEN as usize);
        let header = LeLegacyAdvertisingPduHeader::mut_from_bytes(header_bytes).unwrap();
        header.pdu_type = adv_type as u8;
        header.length = body_len as u8;

        let (adv_address_bytes, adv_data_bytes) = body_bytes.split_at_mut(adv_address.len());
        adv_address_bytes.copy_from_slice(&adv_address);
        adv_data_bytes.copy_from_slice(adv_data);

        Ok(Self { inner })
    }

    /// Returns the PDU as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_le_legacy_advertising_pdu_header_zerocopy() {
        // Test serialization according to BLE spec.
        let header = LeLegacyAdvertisingPduHeader {
            pdu_type: LegacyAdvertisingType::AdvInd as u8,
            length: 42,
        };
        let bytes = header.as_bytes();
        assert_eq!(bytes, &[0x00, 42]);

        // Test parsing according to BLE spec.
        let parsed_header = LeLegacyAdvertisingPduHeader::ref_from_bytes(&[0x01, 24]).unwrap();
        assert_eq!(parsed_header.pdu_type, LegacyAdvertisingType::AdvDirectInd as u8);
        assert_eq!(parsed_header.length, 24);
    }

    #[test]
    fn test_le_legacy_advertising_pdu_construction() {
        let adv_type = LegacyAdvertisingType::AdvInd;
        let adv_address = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let adv_data = &[0x01, 0x02, 0x03];

        let pdu = LeLegacyAdvertisingPdu::new(adv_type, adv_address, adv_data).unwrap();

        // Per Bluetooth Core Spec Vol 4, Part E, Section 7.8.5, the PDU is:
        // Header (2 bytes) + Payload (Length bytes)
        // Header: PDU Type (1 byte), Length (1 byte)
        // Payload: AdvA (6 bytes) + AdvData (Length - 6 bytes)
        let expected_bytes = &[
            0x00, // PDU Type: AdvInd
            0x09, // Length: 6 (addr) + 3 (data)
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, // AdvA
            0x01, 0x02, 0x03, // AdvData
        ];

        assert_eq!(pdu.as_bytes(), expected_bytes);
    }
}
