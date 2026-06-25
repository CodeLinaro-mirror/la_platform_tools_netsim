// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, net::Ipv4Addr};

use netsim_packets::{ArpPacket, ArpPacketBuilder, MacAddr};
use serde::{Deserialize, Serialize};

/// Handles Address Resolution Protocol (ARP).
#[derive(Serialize, Deserialize)]
pub struct ArpTable {
    pub(crate) cache: HashMap<Ipv4Addr, MacAddr>,
}

impl Default for ArpTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ArpTable {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    /// Adds a static entry to the ARP table.
    pub fn add_entry(&mut self, ip: Ipv4Addr, mac: MacAddr) {
        self.cache.insert(ip, mac);
    }

    /// Handles an incoming ARP packet.
    ///
    /// If the packet is a "who-has" request for an IP address owned by this
    /// table, it will generate and return an "is-at" reply packet.
    pub fn handle_packet(&mut self, packet: &ArpPacket) -> Option<Vec<u8>> {
        // We only care about ARP requests for IPv4 over Ethernet.
        if packet.hardware_type.get() != 1 || packet.protocol_type.get() != 0x0800 {
            return None;
        }

        // Opcode 1 is "who-has"
        if packet.opcode.get() == 1 {
            let target_ip = Ipv4Addr::from(packet.target_protocol_addr);
            if let Some(mac_addr) = self.cache.get(&target_ip) {
                // This is a request for an IP we own. Generate a reply.
                let mut reply_buf = [0u8; 28];
                let builder = ArpPacketBuilder::new(&mut reply_buf).unwrap();
                builder
                    .hardware_type(1)
                    .protocol_type(0x0800)
                    .hardware_addr_len(6)
                    .protocol_addr_len(4)
                    .opcode(2) // Reply
                    .sender_hardware_addr(*mac_addr)
                    .sender_protocol_addr(target_ip.octets())
                    .target_hardware_addr(packet.sender_hardware_addr)
                    .target_protocol_addr(packet.sender_protocol_addr)
                    .build();
                return Some(reply_buf.to_vec());
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "tests/arp_tests.rs"]
mod tests;
