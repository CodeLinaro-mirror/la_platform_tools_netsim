// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Defines the ARP (Address Resolution Protocol) header.

use zerocopy::{
    byteorder::NetworkEndian, FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned, U16,
};

use crate::utils::general::ParseResult;

pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;
pub const ARP_HW_ETHERNET: u16 = 1;

/// Represents the ARP Packet for Ethernet and IPv4.
#[derive(FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable, Debug)]
#[repr(C)]
pub struct ArpHeader {
    /// Hardware type (e.g., 1 for Ethernet).
    pub hardware_type: U16<NetworkEndian>,
    /// Protocol type (e.g., 0x0800 for IPv4).
    pub protocol_type: U16<NetworkEndian>,
    /// Hardware address length (e.g., 6 for Ethernet).
    pub hardware_len: u8,
    /// Protocol address length (e.g., 4 for IPv4).
    pub protocol_len: u8,
    /// Operation code (1 for Request, 2 for Reply).
    pub opcode: U16<NetworkEndian>,
    /// Sender hardware address (MAC).
    pub sender_mac: [u8; 6],
    /// Sender protocol address (IP).
    pub sender_ip: [u8; 4],
    /// Target hardware address (MAC).
    pub target_mac: [u8; 6],
    /// Target protocol address (IP).
    pub target_ip: [u8; 4],
}

impl ArpHeader {
    pub fn parse(bytes: &[u8]) -> Option<ParseResult<'_, ArpHeader>> {
        Ref::from_prefix(bytes).ok()
    }
}
