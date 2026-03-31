// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Defines the TCP (Transmission Control Protocol) header using `zerocopy`.

use zerocopy::{
    byteorder::NetworkEndian, FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned, U16,
    U32,
};

use crate::utils::general::ParseResult;

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
    /// A combined field containing the data offset, reserved bits, and TCP
    /// flags.
    pub data_offset_reserved_flags: U16<NetworkEndian>,
    /// The size of the receive window, which specifies the number of window
    /// size units.
    pub window_size: U16<NetworkEndian>,
    /// The checksum of the TCP header and data.
    pub checksum: U16<NetworkEndian>,
    /// If the URG flag is set, this field is an offset from the sequence number
    /// indicating the last urgent data byte.
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

    /// Returns the Data Offset, which is the size of the TCP header in 32-bit
    /// words.
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

    /// Calculates and updates the TCP checksum.
    pub fn update_checksum(&mut self, src_ip: [u8; 4], dst_ip: [u8; 4], payload: &[u8]) {
        self.checksum = U16::new(0);
        let mut sum: u32 = 0;

        // Pseudo-header
        // Source IP
        sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
        sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;
        // Dest IP
        sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
        sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;
        // Zero + Protocol (6 for TCP)
        sum += 6;
        // TCP Length (Header + Data)
        let tcp_len = self.header_length() + payload.len();
        sum += tcp_len as u32;

        // TCP Header
        let header_bytes = self.as_bytes();
        for chunk in header_bytes.chunks(2) {
            if chunk.len() == 2 {
                sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
            } else {
                sum += (chunk[0] as u32) << 8;
            }
        }

        // Payload
        for chunk in payload.chunks(2) {
            if chunk.len() == 2 {
                sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
            } else {
                sum += (chunk[0] as u32) << 8;
            }
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        self.checksum = U16::new(!sum as u16);
    }
}

#[cfg(test)]
#[path = "tcp_tests.rs"]
mod tests;
