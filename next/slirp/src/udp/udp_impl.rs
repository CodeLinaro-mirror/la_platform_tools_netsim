// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, net::SocketAddr};

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::{ConnectionArgs, SlirpResponse, UdpConnectionArgs, UdpConnectionInfo};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UdpFlow {
    pub guest_addr: SocketAddr,
    pub original_dest: SocketAddr,
}

/// Manages UDP flows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UdpManager {
    // Maps guest_addr -> (conn_id, real_dest_addr)
    pub(crate) flows: HashMap<SocketAddr, (u64, SocketAddr)>,
    // Maps conn_id -> UdpFlow
    pub(crate) id_to_flow: HashMap<u64, UdpFlow>,
    pub(crate) next_flow_id: u64,
}

impl Default for UdpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpManager {
    pub fn new() -> Self {
        Self {
            flows: HashMap::new(),
            id_to_flow: HashMap::new(),
            next_flow_id: 1 << 63, // Start UDP IDs with MSB set
        }
    }

    pub fn get_connections(&self) -> impl Iterator<Item = UdpConnectionInfo> + '_ {
        self.flows.iter().map(|(guest_addr, (_, real_dest))| UdpConnectionInfo {
            local_addr: *guest_addr,
            peer_addr: *real_dest,
        })
    }

    /// Handles an incoming UDP packet from the guest.
    ///
    /// If `redirect_dest` is provided, the connection will be established to
    /// that address instead of the packet's original destination, but
    /// replies will still appear to come from the original destination.
    pub fn handle_packet(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        packet: &[u8],
        source: SocketAddr,
        dest: SocketAddr,
        redirect_dest: Option<SocketAddr>,
    ) {
        let real_dest = redirect_dest.unwrap_or(dest);
        let (id, _) = self.flows.entry(source).or_insert_with(|| {
            let id = self.next_flow_id;
            self.next_flow_id += 1;
            let conn_info = UdpConnectionArgs {
                destination: real_dest,
                guest_ip: source.ip(),
                guest_port: source.port(),
            };
            responses.push(SlirpResponse::EstablishConnection(id, ConnectionArgs::Udp(conn_info)));

            self.id_to_flow.insert(id, UdpFlow { guest_addr: source, original_dest: dest });

            (id, real_dest)
        });

        responses.push(SlirpResponse::WriteToConnection(*id, Bytes::copy_from_slice(packet)));
    }

    /// Translates a host reply and returns the original destination (to be used
    /// as source), the guest address (destination), and the payload.
    pub fn handle_reply(
        &self,
        conn_id: u64,
        data: &[u8],
    ) -> Option<(SocketAddr, SocketAddr, Vec<u8>)> {
        self.id_to_flow
            .get(&conn_id)
            .map(|flow| (flow.original_dest, flow.guest_addr, data.to_vec()))
    }

    /// Removes a flow by its connection ID.
    pub fn remove_flow(&mut self, conn_id: u64) {
        if let Some(flow) = self.id_to_flow.remove(&conn_id) {
            self.flows.remove(&flow.guest_addr);
        }
    }
}

#[cfg(test)]
#[path = "tests/udp_tests.rs"]
mod tests;
