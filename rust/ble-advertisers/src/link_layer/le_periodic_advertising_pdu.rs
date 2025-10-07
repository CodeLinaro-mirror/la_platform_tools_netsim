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

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(
    FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable, Debug, PartialEq, Eq, Clone, Copy,
)]
#[repr(C, packed)]
pub struct LePeriodicAdvertisingPdu {
    pub advertising_address_type: u8,
    pub sid: u8,
    pub tx_power: u8,
    pub advertising_interval: u16,
}

impl LePeriodicAdvertisingPdu {
    pub fn parse(bytes: &[u8]) -> Result<(&Self, &[u8]), &'static str> {
        let (packet, tail) = Self::ref_from_prefix(bytes).map_err(|_| "Not enough bytes")?;
        Ok((packet, tail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zerocopy_layout_and_parse() {
        let pdu = LePeriodicAdvertisingPdu {
            advertising_address_type: 1,
            sid: 3,
            tx_power: 127,
            advertising_interval: 0x1234,
        };

        // Test serialization
        let bytes = pdu.as_bytes();
        let expected_bytes = &[
            1,   // advertising_address_type
            3,   // sid
            127, // tx_power
            0x34, 0x12, // advertising_interval (little-endian)
        ];
        assert_eq!(bytes, expected_bytes);

        // Test parsing
        let (parsed_pdu, tail) = LePeriodicAdvertisingPdu::parse(expected_bytes).unwrap();
        assert_eq!(parsed_pdu, &pdu);
        assert!(tail.is_empty());
    }
}
