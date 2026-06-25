// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, net::Ipv6Addr};

use netsim_packets::{
    MacAddr, NeighborAdvertisement, NeighborAdvertisementBuilder, NeighborSolicitation,
};
use serde::{Deserialize, Serialize};

/// Handles Neighbor Discovery Protocol (NDP) for IPv6.
#[derive(Serialize, Deserialize)]
pub struct NdpTable {
    pub(crate) cache: HashMap<Ipv6Addr, MacAddr>,
}

impl Default for NdpTable {
    fn default() -> Self {
        Self::new()
    }
}

impl NdpTable {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    /// Adds a static entry to the NDP table.
    pub fn add_entry(&mut self, ip: Ipv6Addr, mac: MacAddr) {
        self.cache.insert(ip, mac);
    }

    /// Handles an incoming Neighbor Solicitation packet.
    ///
    /// If the packet is a request for an IP address owned by this
    /// table, it will generate and return a Neighbor Advertisement reply
    /// packet.
    pub fn handle_packet(&mut self, packet: &NeighborSolicitation) -> Option<Vec<u8>> {
        let target_addr = Ipv6Addr::from(packet.target_addr);
        if self.cache.contains_key(&target_addr) {
            // This is a request for an IP we own. Generate a reply.
            let mut reply_buf = [0u8; std::mem::size_of::<NeighborAdvertisement>()];
            let builder = NeighborAdvertisementBuilder::new(&mut reply_buf).unwrap();
            builder
                .flags(0b01100000) // Router, Solicited, Override
                .target_addr(target_addr.octets())
                .build();
            return Some(reply_buf.to_vec());
        }
        None
    }
}

#[cfg(test)]
#[path = "tests/ndp_tests.rs"]
mod tests;
