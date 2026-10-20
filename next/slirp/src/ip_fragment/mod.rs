// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

mod ip_fragment_impl;
#[cfg(test)]
mod tests;

use bytes::Bytes;
pub use ip_fragment_impl::{MTU, Reassembler, fragment};
use netsim_packets::Ipv4Header;
use zerocopy::FromBytes;

use crate::{
    SlirpResponse,
    packet::{IpPacket, NetworkPacket, ParsedPacket},
};

/// Post-processes a list of SlirpResponses, fragmenting any outgoing IP packets
/// that exceed the MTU.
pub fn fragment_outgoing_packets(responses: &mut Vec<SlirpResponse>) {
    let mut i = 0;
    while i < responses.len() {
        let parsed = match &responses[i] {
            SlirpResponse::Packet(packet_bytes) => {
                ParsedPacket::parse(packet_bytes).and_then(|p| {
                    let eth_header_len = match p.ethernet {
                        netsim_packets::EthernetPacket::Untagged { .. } => 14,
                        netsim_packets::EthernetPacket::Vlan { .. } => 18,
                    };
                    match p.network {
                        Some(NetworkPacket::Ip(IpPacket::V4(ref ip_header, _))) => {
                            Some((packet_bytes, eth_header_len, ip_header.header_length()))
                        }
                        _ => None,
                    }
                })
            }
            _ => None,
        };
        if let Some((packet_bytes, eth_header_len, ip_header_len)) = parsed {
            let ip_packet_len = packet_bytes.len() - eth_header_len;

            if ip_packet_len > MTU {
                log::trace!("Fragmenting outgoing packet of size {}", packet_bytes.len());

                let ip_fragments = fragment(&packet_bytes[eth_header_len..]);

                let mut eth_fragments = Vec::with_capacity(ip_fragments.len());
                for ip_frag in ip_fragments {
                    let mut eth_frag = Vec::with_capacity(eth_header_len + ip_frag.len());
                    eth_frag.extend_from_slice(&packet_bytes[..eth_header_len]);
                    eth_frag.extend_from_slice(&ip_frag);

                    {
                        let ip_header_slice =
                            &mut eth_frag[eth_header_len..eth_header_len + ip_header_len];
                        let checksum = netsim_packets::ipv4_checksum(ip_header_slice);
                        let mut_ip_header = Ipv4Header::mut_from_bytes(ip_header_slice).unwrap();
                        mut_ip_header.header_checksum.set(checksum);
                    }

                    eth_fragments.push(SlirpResponse::Packet(Bytes::copy_from_slice(&eth_frag)));
                }

                let num_frags = eth_fragments.len();
                responses.splice(i..=i, eth_fragments);
                i += num_frags;
                continue;
            }
        }
        i += 1;
    }
}
