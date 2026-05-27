// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Defines the UDP (User Datagram Protocol) header using `zerocopy`.

use crate::util::ParseResult;
use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Ref, U16, Unaligned, byteorder::NetworkEndian,
};

/// Represents the UDP header.
#[derive(FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable, Debug)]
#[repr(C)]
pub struct UdpHeader {
    /// The source port number.
    pub source_port: U16<NetworkEndian>,
    /// The destination port number.
    pub dest_port: U16<NetworkEndian>,
    /// The length of the UDP header and data in bytes.
    pub length: U16<NetworkEndian>,
    /// The UDP checksum.
    pub checksum: U16<NetworkEndian>,
}

impl UdpHeader {
    /// Parses a `UdpHeader` from the beginning of the given byte slice.
    ///
    /// Returns a reference to the header and a slice for the remaining bytes (the UDP payload).
    pub fn parse(bytes: &[u8]) -> Option<ParseResult<UdpHeader>> {
        Ref::from_prefix(bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_header_parsing() {
        // Sample UDP header: source port 12345, dest port 80, length 28, checksum 0xABCD
        let bytes: [u8; 8] = [0x30, 0x39, 0x00, 0x50, 0x00, 0x1C, 0xAB, 0xCD];
        let (header, rest) = UdpHeader::parse(&bytes).expect("Failed to parse UDP header");

        assert_eq!(header.source_port.get(), 12345);
        assert_eq!(header.dest_port.get(), 80);
        assert_eq!(header.length.get(), 28);
        assert_eq!(header.checksum.get(), 0xABCD);
        assert!(rest.is_empty());
    }

    #[test]
    fn test_udp_header_with_payload() {
        let bytes: [u8; 12] =
            [0x30, 0x39, 0x00, 0x50, 0x00, 0x1C, 0xAB, 0xCD, 0xDE, 0xAD, 0xBE, 0xEF];
        let (header, rest) = UdpHeader::parse(&bytes).expect("Failed to parse UDP header");

        assert_eq!(header.source_port.get(), 12345);
        assert_eq!(rest.len(), 4);
        assert_eq!(rest, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_udp_header_too_short() {
        let bytes: [u8; 7] = [0x30, 0x39, 0x00, 0x50, 0x00, 0x1C, 0xAB];
        assert!(UdpHeader::parse(&bytes).is_none());
    }
}
