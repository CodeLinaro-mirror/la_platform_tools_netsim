// Copyright 2025 The Android Open Source Project

//! Defines the UDP (User Datagram Protocol) header using `zerocopy`.

use crate::utils::general::ParseResult;
use zerocopy::{
    byteorder::NetworkEndian, FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned, U16,
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
    pub fn parse(bytes: &[u8]) -> Option<ParseResult<'_, UdpHeader>> {
        Ref::from_prefix(bytes).ok()
    }
}

#[cfg(test)]
#[path = "udp_tests.rs"]
mod tests;
