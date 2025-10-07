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

//! A LE Scan Request PDU.

use crate::link_layer::types::{Address, LegacyAdvertisingType};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// The header of a LE Scan Request PDU.
#[derive(FromBytes, Unaligned, KnownLayout, Immutable)]
#[repr(C, packed)]
struct LeScanReqPduHeader {
    pdu_type: u8,
    length: u8,
}

/// A LE Scan Request PDU, as defined in the Bluetooth Core Specification,
/// Vol 6, Part B, Section 2.3.3.1.
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
#[repr(C, packed)]
pub struct LeScanReqPdu {
    /// The scanner's address.
    pub scanning_address: Address,
    /// The advertiser's address.
    pub advertising_address: Address,
}

impl LeScanReqPdu {
    /// The fixed length of a scan request PDU body.
    pub const BODY_LEN: u8 = 12;

    /// Parses a byte slice into a `LeScanReqPdu`.
    ///
    /// Returns an error if the byte slice is not a valid scan request PDU.
    pub fn parse(bytes: &[u8]) -> Result<&Self, String> {
        let (header, body_bytes) = LeScanReqPduHeader::ref_from_prefix(bytes)
            .map_err(|_| "Not enough bytes for ScanReq PDU header")?;
        if header.pdu_type != LegacyAdvertisingType::ScanReq as u8 {
            return Err("Invalid PDU type for ScanReq".to_string());
        }
        if header.length != Self::BODY_LEN {
            return Err("Invalid length for ScanReq PDU".to_string());
        }
        let (body, _) = Self::ref_from_prefix(body_bytes)
            .map_err(|_| "Not enough bytes for ScanReq PDU body".to_string())?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_le_scan_req_pdu_parse_success() {
        let scanning_address = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let advertising_address = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];

        // Construct the PDU bytes according to the spec.
        // Header (2 bytes) + Body (12 bytes)
        let mut pdu_bytes = vec![LegacyAdvertisingType::ScanReq as u8, LeScanReqPdu::BODY_LEN];
        pdu_bytes.extend_from_slice(&scanning_address);
        pdu_bytes.extend_from_slice(&advertising_address);

        let pdu = LeScanReqPdu::parse(&pdu_bytes).unwrap();

        assert_eq!(pdu.scanning_address, scanning_address);
        assert_eq!(pdu.advertising_address, advertising_address);
    }

    #[test]
    fn test_le_scan_req_pdu_zerocopy_layout() {
        let scanning_address = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let advertising_address = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let pdu = LeScanReqPdu { scanning_address, advertising_address };

        // Test serialization
        let bytes = pdu.as_bytes();
        let mut expected_bytes = vec![];
        expected_bytes.extend_from_slice(&scanning_address);
        expected_bytes.extend_from_slice(&advertising_address);
        assert_eq!(bytes, &expected_bytes[..]);

        // Test parsing
        let parsed_pdu = LeScanReqPdu::ref_from_bytes(&expected_bytes).unwrap();
        assert_eq!(parsed_pdu.scanning_address, scanning_address);
        assert_eq!(parsed_pdu.advertising_address, advertising_address);
    }

    #[test]
    fn test_parse_invalid_pdu_type() {
        let pdu_bytes = vec![
            LegacyAdvertisingType::AdvInd as u8,
            LeScanReqPdu::BODY_LEN,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        assert!(LeScanReqPdu::parse(&pdu_bytes).is_err());
    }

    #[test]
    fn test_parse_invalid_length() {
        let pdu_bytes =
            vec![LegacyAdvertisingType::ScanReq as u8, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(LeScanReqPdu::parse(&pdu_bytes).is_err());
    }

    #[test]
    fn test_parse_not_enough_bytes() {
        let pdu_bytes =
            vec![LegacyAdvertisingType::ScanReq as u8, LeScanReqPdu::BODY_LEN, 0, 0, 0, 0, 0, 0];
        assert!(LeScanReqPdu::parse(&pdu_bytes).is_err());
    }
}
