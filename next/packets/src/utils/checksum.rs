// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Checksum utility functions for network packets.

use std::net::{Ipv4Addr, Ipv6Addr};

/// Calculates the IPv4 checksum.
///
/// The checksum is calculated by summing the 16-bit words of the data,
/// adding any carry-over back at the end, and then taking the one's
/// complement.
pub fn ipv4_checksum(data: &[u8]) -> u16 {
    fold_checksum(sum_slice(data))
}

fn checksum_v4_generic(
    packet: &[u8],
    src_addr: Ipv4Addr,
    dst_addr: Ipv4Addr,
    protocol: u16,
) -> u16 {
    let mut sum = 0u64;

    // Pseudo-header
    let src_bytes = src_addr.octets();
    let dst_bytes = dst_addr.octets();
    sum += u64::from(u16::from_be_bytes([src_bytes[0], src_bytes[1]]));
    sum += u64::from(u16::from_be_bytes([src_bytes[2], src_bytes[3]]));
    sum += u64::from(u16::from_be_bytes([dst_bytes[0], dst_bytes[1]]));
    sum += u64::from(u16::from_be_bytes([dst_bytes[2], dst_bytes[3]]));
    sum += u64::from(protocol);
    sum += u64::from(packet.len() as u16);

    sum += sum_slice(packet);

    fold_checksum(sum)
}

/// Calculates the TCP checksum for IPv4.
pub fn tcp_checksum(tcp_packet: &[u8], src_addr: Ipv4Addr, dst_addr: Ipv4Addr) -> u16 {
    checksum_v4_generic(tcp_packet, src_addr, dst_addr, 6)
}

/// Calculates the UDP checksum for IPv4.
pub fn udp_checksum(udp_packet: &[u8], src_addr: Ipv4Addr, dst_addr: Ipv4Addr) -> u16 {
    let cksum = checksum_v4_generic(udp_packet, src_addr, dst_addr, 17);
    if cksum == 0 { 0xFFFF } else { cksum }
}

fn checksum_v6_generic(
    packet: &[u8],
    src_addr: Ipv6Addr,
    dst_addr: Ipv6Addr,
    protocol: u16,
) -> u16 {
    let mut sum = 0u64;

    // Pseudo-header
    let src_bytes = src_addr.octets();
    for i in (0..16).step_by(2) {
        sum += u64::from(u16::from_be_bytes([src_bytes[i], src_bytes[i + 1]]));
    }
    let dst_bytes = dst_addr.octets();
    for i in (0..16).step_by(2) {
        sum += u64::from(u16::from_be_bytes([dst_bytes[i], dst_bytes[i + 1]]));
    }
    let len = packet.len() as u64;
    sum += len >> 16;
    sum += len & 0xFFFF;
    sum += u64::from(protocol);

    sum += sum_slice(packet);

    fold_checksum(sum)
}

/// Calculates the TCP checksum for IPv6.
pub fn tcp_checksum_v6(tcp_packet: &[u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> u16 {
    checksum_v6_generic(tcp_packet, src_addr, dst_addr, 6)
}

/// Calculates the UDP checksum for IPv6.
pub fn udp_checksum_v6(udp_packet: &[u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> u16 {
    let cksum = checksum_v6_generic(udp_packet, src_addr, dst_addr, 17);
    if cksum == 0 { 0xFFFF } else { cksum }
}

/// Calculates the ICMPv6 checksum using the IPv6 pseudo-header.
pub fn icmpv6_checksum(icmpv6_packet: &[u8], src_addr: Ipv6Addr, dst_addr: Ipv6Addr) -> u16 {
    checksum_v6_generic(icmpv6_packet, src_addr, dst_addr, 58)
}

fn sum_slice(data: &[u8]) -> u64 {
    let mut sum = 0u64;
    for chunk in data.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], 0])
        };
        sum += u64::from(word);
    }
    sum
}

fn fold_checksum(mut sum: u64) -> u16 {
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}
