// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! A TCP packet builder.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use zerocopy::FromBytes;

use super::tcp::TcpHeader;

pub struct TcpBuilder<'a> {
    buffer: &'a mut [u8],
    src_addr: IpAddr,
    dst_addr: IpAddr,
    data_offset: u8, // in 32-bit words
    payload_len: usize,
}

impl<'a> TcpBuilder<'a> {
    fn new_generic(buffer: &'a mut [u8], src_addr: IpAddr, dst_addr: IpAddr) -> Option<Self> {
        if buffer.len() < std::mem::size_of::<TcpHeader>() {
            return None;
        }
        let data_offset = 5; // Default: 20 bytes
        let mut builder = Self { buffer, src_addr, dst_addr, data_offset, payload_len: 0 };
        builder.header_mut().source_port.set(0);
        builder.header_mut().dest_port.set(0);
        builder.header_mut().sequence_num.set(0);
        builder.header_mut().ack_num.set(0);
        builder.header_mut().data_offset_reserved_flags.set((data_offset as u16) << 12);
        builder.header_mut().window_size.set(65535);
        builder.header_mut().checksum.set(0);
        builder.header_mut().urgent_ptr.set(0);
        Some(builder)
    }

    pub fn new(buffer: &'a mut [u8], src_addr: Ipv4Addr, dst_addr: Ipv4Addr) -> Option<Self> {
        Self::new_generic(buffer, IpAddr::V4(src_addr), IpAddr::V4(dst_addr))
    }

    pub fn new_v6(buffer: &'a mut [u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> Option<Self> {
        Self::new_generic(buffer, IpAddr::V6(src_addr), IpAddr::V6(dst_addr))
    }

    fn header_mut(&mut self) -> &mut TcpHeader {
        TcpHeader::mut_from_bytes(&mut self.buffer[..std::mem::size_of::<TcpHeader>()]).unwrap()
    }

    /// Sets the data offset (header length) in 32-bit words.
    ///
    /// # Note
    /// This should be called *before* writing any payload data (via
    /// `payload_mut()`), as changing the data offset shifts the start of
    /// the payload area.
    pub fn data_offset(&mut self, offset_words: u8) -> &mut Self {
        let clamped_offset = offset_words.clamp(5, 15);
        self.data_offset = clamped_offset;
        let current = self.header_mut().data_offset_reserved_flags.get();
        // Preserve only the 9 flags (lower 9 bits: 0x01FF), force Reserved (bits 9-11)
        // to 0, and set the new Data Offset (bits 12-15).
        self.header_mut()
            .data_offset_reserved_flags
            .set((current & 0x01FF) | ((clamped_offset as u16) << 12));
        self
    }

    pub fn source_port(&mut self, port: u16) -> &mut Self {
        self.header_mut().source_port.set(port);
        self
    }

    pub fn dest_port(&mut self, port: u16) -> &mut Self {
        self.header_mut().dest_port.set(port);
        self
    }

    pub fn sequence_num(&mut self, seq: u32) -> &mut Self {
        self.header_mut().sequence_num.set(seq);
        self
    }

    pub fn ack_num(&mut self, ack: u32) -> &mut Self {
        self.header_mut().ack_num.set(ack);
        self
    }

    pub fn flags(&mut self, flags: u16) -> &mut Self {
        let current = self.header_mut().data_offset_reserved_flags.get();
        // Preserve only Data Offset (bits 12-15), force Reserved (bits 9-11) to 0,
        // and set the new Flags (masked to 9 bits: 0x01FF).
        self.header_mut().data_offset_reserved_flags.set((current & 0xF000) | (flags & 0x01FF));
        self
    }

    pub fn window_size(&mut self, window_size: u16) -> &mut Self {
        self.header_mut().window_size.set(window_size);
        self
    }

    pub fn urgent_ptr(&mut self, urgent_ptr: u16) -> &mut Self {
        self.header_mut().urgent_ptr.set(urgent_ptr);
        self
    }

    pub fn payload_mut(&mut self) -> Option<&mut [u8]> {
        let total_header_len = self.data_offset as usize * 4;
        if total_header_len > self.buffer.len() {
            None
        } else {
            Some(&mut self.buffer[total_header_len..])
        }
    }

    /// Returns a mutable reference to the TCP options space.
    ///
    /// Returns `None` if `data_offset` is 5 (no options) or if the header
    /// length exceeds the buffer size.
    pub fn options_mut(&mut self) -> Option<&mut [u8]> {
        let total_header_len = self.data_offset as usize * 4;
        if total_header_len > self.buffer.len() || self.data_offset <= 5 {
            None
        } else {
            Some(&mut self.buffer[std::mem::size_of::<TcpHeader>()..total_header_len])
        }
    }

    pub fn payload<'b>(&'b mut self, payload: &[u8]) -> Option<&'b mut Self> {
        let total_header_len = self.data_offset as usize * 4;
        if total_header_len + payload.len() > self.buffer.len()
            || total_header_len + payload.len() > u16::MAX as usize
        {
            return None;
        }
        self.payload_len = payload.len();
        self.payload_mut()?[..payload.len()].copy_from_slice(payload);
        Some(self)
    }

    pub fn payload_len(&mut self, len: usize) -> &mut Self {
        self.payload_len = len;
        self
    }

    pub fn build(mut self) -> Option<usize> {
        self.header_mut().checksum.set(0);
        let total_header_len = self.data_offset as usize * 4;
        if total_header_len + self.payload_len > self.buffer.len()
            || total_header_len + self.payload_len > u16::MAX as usize
        {
            return None;
        }
        let tcp_segment = &self.buffer[..total_header_len + self.payload_len];
        let checksum = match (self.src_addr, self.dst_addr) {
            (IpAddr::V4(src), IpAddr::V4(dst)) => {
                crate::utils::checksum::tcp_checksum(tcp_segment, src, dst)
            }
            (IpAddr::V6(src), IpAddr::V6(dst)) => {
                crate::utils::checksum::tcp_checksum_v6(tcp_segment, src, dst)
            }
            _ => return None,
        };
        self.header_mut().checksum.set(checksum);
        Some(total_header_len + self.payload_len)
    }
}
