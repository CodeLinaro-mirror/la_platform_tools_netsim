// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! A UDP packet builder.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use zerocopy::FromBytes;

use super::udp::UdpHeader;

/// A builder for a UDP packet.
pub struct UdpBuilder<'a> {
    buffer: &'a mut [u8],
    src_addr: IpAddr,
    dst_addr: IpAddr,
    payload_len: usize,
}

impl<'a> UdpBuilder<'a> {
    fn new_generic(
        buffer: &'a mut [u8],
        src_addr: IpAddr,
        dst_addr: IpAddr,
        src_port: u16,
        dst_port: u16,
    ) -> Option<Self> {
        if buffer.len() < std::mem::size_of::<UdpHeader>() {
            return None;
        }
        let mut builder = Self { buffer, src_addr, dst_addr, payload_len: 0 };
        builder.header_mut().source_port.set(src_port);
        builder.header_mut().dest_port.set(dst_port);
        builder.header_mut().length.set(std::mem::size_of::<UdpHeader>() as u16);
        builder.header_mut().checksum.set(0);
        Some(builder)
    }

    /// Creates a new `UdpBuilder` for IPv4.
    pub fn new(
        buffer: &'a mut [u8],
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
    ) -> Option<Self> {
        Self::new_generic(buffer, IpAddr::V4(src_addr), IpAddr::V4(dst_addr), src_port, dst_port)
    }

    /// Creates a new `UdpBuilder` for IPv6.
    pub fn new_v6(
        buffer: &'a mut [u8],
        src_addr: Ipv6Addr,
        dst_addr: Ipv6Addr,
        src_port: u16,
        dst_port: u16,
    ) -> Option<Self> {
        Self::new_generic(buffer, IpAddr::V6(src_addr), IpAddr::V6(dst_addr), src_port, dst_port)
    }

    fn header_mut(&mut self) -> &mut UdpHeader {
        UdpHeader::mut_from_bytes(&mut self.buffer[..std::mem::size_of::<UdpHeader>()]).unwrap()
    }

    /// Returns a mutable reference to the payload.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[std::mem::size_of::<UdpHeader>()..]
    }

    /// Sets the payload data.
    pub fn payload<'b>(&'b mut self, payload: &[u8]) -> Option<&'b mut Self> {
        let total_len = std::mem::size_of::<UdpHeader>() + payload.len();
        if total_len > self.buffer.len() || total_len > u16::MAX as usize {
            return None;
        }
        self.payload_len = payload.len();
        self.payload_mut()[..payload.len()].copy_from_slice(payload);
        Some(self)
    }

    /// Sets the payload length.
    pub fn payload_len(&mut self, len: usize) -> &mut Self {
        self.payload_len = len;
        self
    }

    /// Builds the UDP packet.
    pub fn build(mut self) -> Option<usize> {
        self.header_mut().checksum.set(0);
        let total_len = std::mem::size_of::<UdpHeader>() + self.payload_len;
        if total_len > self.buffer.len() || total_len > u16::MAX as usize {
            return None;
        }
        self.header_mut().length.set(total_len as u16);
        let udp_segment = &self.buffer[..total_len];
        let checksum = match (self.src_addr, self.dst_addr) {
            (IpAddr::V4(src), IpAddr::V4(dst)) => {
                crate::utils::checksum::udp_checksum(udp_segment, src, dst)
            }
            (IpAddr::V6(src), IpAddr::V6(dst)) => {
                crate::utils::checksum::udp_checksum_v6(udp_segment, src, dst)
            }
            _ => return None,
        };
        self.header_mut().checksum.set(checksum);
        Some(total_len)
    }
}
