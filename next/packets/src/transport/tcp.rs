// Copyright 2025 The Android Open Source Project

//! Defines the TCP (Transmission Control Protocol) header using `zerocopy`.

use crate::utils::general::ParseResult;
use zerocopy::{
    byteorder::NetworkEndian, FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned, U16,
    U32,
};

/// Represents the TCP header.
#[derive(FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable, Debug)]
#[repr(C)]
pub struct TcpHeader {
    /// The source port number.
    pub source_port: U16<NetworkEndian>,
    /// The destination port number.
    pub dest_port: U16<NetworkEndian>,
    /// The sequence number.
    pub sequence_num: U32<NetworkEndian>,
    /// The acknowledgment number, if the ACK flag is set.
    pub ack_num: U32<NetworkEndian>,
    /// A combined field containing the data offset, reserved bits, and TCP flags.
    pub data_offset_reserved_flags: U16<NetworkEndian>,
    /// The size of the receive window, which specifies the number of window size units.
    pub window_size: U16<NetworkEndian>,
    /// The checksum of the TCP header and data.
    pub checksum: U16<NetworkEndian>,
    /// If the URG flag is set, this field is an offset from the sequence number indicating the last urgent data byte.
    pub urgent_ptr: U16<NetworkEndian>,
}

/// TCP flags.
pub mod flags {
    pub const FIN: u16 = 1;
    pub const SYN: u16 = 1 << 1;
    pub const RST: u16 = 1 << 2;
    pub const PSH: u16 = 1 << 3;
    pub const ACK: u16 = 1 << 4;
    pub const URG: u16 = 1 << 5;
    pub const ECE: u16 = 1 << 6;
    pub const CWR: u16 = 1 << 7;
    pub const NS: u16 = 1 << 8;
}

impl TcpHeader {
    /// Parses a `TcpHeader` from the beginning of the given byte slice.
    pub fn parse(bytes: &[u8]) -> Option<ParseResult<'_, TcpHeader>> {
        let (header, _) = Ref::<&[u8], TcpHeader>::from_prefix(bytes).ok()?;
        let header_len = header.header_length();
        if bytes.len() < header_len {
            return None;
        }
        Some((header, &bytes[header_len..]))
    }

    /// Returns the Data Offset, which is the size of the TCP header in 32-bit words.
    pub fn data_offset(&self) -> u8 {
        (self.data_offset_reserved_flags.get() >> 12) as u8
    }

    /// Returns the length of the TCP header in bytes.
    pub fn header_length(&self) -> usize {
        self.data_offset() as usize * 4
    }

    /// Returns the TCP flags as a u16 value.
    pub fn flags(&self) -> u16 {
        self.data_offset_reserved_flags.get() & 0x1FF
    }

    /// Returns true if the FIN flag is set.
    pub fn fin(&self) -> bool {
        (self.flags() & flags::FIN) != 0
    }

    /// Returns true if the SYN flag is set.
    pub fn syn(&self) -> bool {
        (self.flags() & flags::SYN) != 0
    }

    /// Returns true if the RST flag is set.
    pub fn rst(&self) -> bool {
        (self.flags() & flags::RST) != 0
    }

    /// Returns true if the PSH flag is set.
    pub fn psh(&self) -> bool {
        (self.flags() & flags::PSH) != 0
    }

    /// Returns true if the ACK flag is set.
    pub fn ack(&self) -> bool {
        (self.flags() & flags::ACK) != 0
    }

    /// Returns true if the URG flag is set.
    pub fn urg(&self) -> bool {
        (self.flags() & flags::URG) != 0
    }
}

#[cfg(test)]
#[path = "tcp_tests.rs"]
mod tests;
