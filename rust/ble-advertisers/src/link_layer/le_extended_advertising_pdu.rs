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
pub struct LeExtendedAdvertisingPdu {
    pub advertising_address_type: u8,
    pub target_address_type: u8,
    connectable_and_scannable_and_directed_and_reserved: u8,
    pub sid: u8,
    pub tx_power: u8,
    pub primary_phy: u8,
    pub secondary_phy: u8,
    pub periodic_advertising_interval: u16,
}

impl LeExtendedAdvertisingPdu {
    pub fn get_connectable(&self) -> u8 {
        let mask = (1u64 << 1) - 1;
        ((self.connectable_and_scannable_and_directed_and_reserved as u64 >> 0) & mask) as u8
    }

    pub fn set_connectable(&mut self, value: u8) {
        let mask = (((1u64 << 1) - 1) << 0) as u8;
        self.connectable_and_scannable_and_directed_and_reserved =
            (self.connectable_and_scannable_and_directed_and_reserved & !mask)
                | ((value as u8) << 0) & mask;
    }

    pub fn get_scannable(&self) -> u8 {
        let mask = (1u64 << 1) - 1;
        ((self.connectable_and_scannable_and_directed_and_reserved as u64 >> 1) & mask) as u8
    }

    pub fn set_scannable(&mut self, value: u8) {
        let mask = (((1u64 << 1) - 1) << 1) as u8;
        self.connectable_and_scannable_and_directed_and_reserved =
            (self.connectable_and_scannable_and_directed_and_reserved & !mask)
                | ((value as u8) << 1) & mask;
    }

    pub fn get_directed(&self) -> u8 {
        let mask = (1u64 << 1) - 1;
        ((self.connectable_and_scannable_and_directed_and_reserved as u64 >> 2) & mask) as u8
    }

    pub fn set_directed(&mut self, value: u8) {
        let mask = (((1u64 << 1) - 1) << 2) as u8;
        self.connectable_and_scannable_and_directed_and_reserved =
            (self.connectable_and_scannable_and_directed_and_reserved & !mask)
                | ((value as u8) << 2) & mask;
    }

    pub fn get_reserved(&self) -> u8 {
        let mask = (1u64 << 5) - 1;
        ((self.connectable_and_scannable_and_directed_and_reserved as u64 >> 3) & mask) as u8
    }

    pub fn set_reserved(&mut self, value: u8) {
        let mask = (((1u64 << 5) - 1) << 3) as u8;
        self.connectable_and_scannable_and_directed_and_reserved =
            (self.connectable_and_scannable_and_directed_and_reserved & !mask)
                | ((value as u8) << 3) & mask;
    }

    pub fn parse(bytes: &[u8]) -> Result<(&Self, &[u8]), &'static str> {
        let (packet, tail) = Self::ref_from_prefix(bytes).map_err(|_| "Not enough bytes")?;
        Ok((packet, tail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitfield_accessors() {
        let mut pdu = LeExtendedAdvertisingPdu {
            advertising_address_type: 0,
            target_address_type: 0,
            connectable_and_scannable_and_directed_and_reserved: 0,
            sid: 0,
            tx_power: 0,
            primary_phy: 0,
            secondary_phy: 0,
            periodic_advertising_interval: 0,
        };

        // Test connectable
        pdu.set_connectable(1);
        assert_eq!(pdu.get_connectable(), 1);
        assert_eq!(pdu.connectable_and_scannable_and_directed_and_reserved, 0b00000001);

        // Test scannable
        pdu.set_scannable(1);
        assert_eq!(pdu.get_scannable(), 1);
        assert_eq!(pdu.connectable_and_scannable_and_directed_and_reserved, 0b00000011);

        // Test directed
        pdu.set_directed(1);
        assert_eq!(pdu.get_directed(), 1);
        assert_eq!(pdu.connectable_and_scannable_and_directed_and_reserved, 0b00000111);

        // Test reserved
        pdu.set_reserved(0b11111);
        assert_eq!(pdu.get_reserved(), 0b11111);
        assert_eq!(pdu.connectable_and_scannable_and_directed_and_reserved, 0b11111111);

        // Test clearing a bit
        pdu.set_scannable(0);
        assert_eq!(pdu.get_scannable(), 0);
        assert_eq!(pdu.connectable_and_scannable_and_directed_and_reserved, 0b11111101);
    }

    #[test]
    fn test_zerocopy_layout_and_parse() {
        let mut pdu = LeExtendedAdvertisingPdu {
            advertising_address_type: 1,
            target_address_type: 2,
            connectable_and_scannable_and_directed_and_reserved: 0,
            sid: 3,
            tx_power: 127,
            primary_phy: 4,
            secondary_phy: 5,
            periodic_advertising_interval: 0x1234,
        };
        pdu.set_connectable(1);
        pdu.set_scannable(0);
        pdu.set_directed(1);
        pdu.set_reserved(0);

        let expected_bitfield = 0b00000101;

        // Test serialization
        let bytes = pdu.as_bytes();
        let expected_bytes = &[
            1, // advertising_address_type
            2, // target_address_type
            expected_bitfield,
            3,   // sid
            127, // tx_power
            4,   // primary_phy
            5,   // secondary_phy
            0x34,
            0x12, // periodic_advertising_interval (little-endian)
        ];
        assert_eq!(bytes, expected_bytes);

        // Test parsing
        let (parsed_pdu, tail) = LeExtendedAdvertisingPdu::parse(expected_bytes).unwrap();
        assert_eq!(parsed_pdu, &pdu);
        assert!(tail.is_empty());
        assert_eq!(parsed_pdu.get_connectable(), 1);
        assert_eq!(parsed_pdu.get_scannable(), 0);
        assert_eq!(parsed_pdu.get_directed(), 1);
    }
}
