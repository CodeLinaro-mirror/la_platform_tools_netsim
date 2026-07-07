// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! ICMP protocol implementation.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};

use bytes::Bytes;
use netsim_packets::{
    EthernetFrame, IcmpEcho, IcmpHeader, IcmpType, Ipv4Builder, Ipv4Header, MacAddr,
    UnreachableCode,
};
use zerocopy::FromBytes;

use crate::{
    Config, ConnectionArgs, IcmpConnectionArgs, SlirpResponse,
    packet::{IpPacket, NetworkPacket, ParsedPacket, TransportPacket},
};

pub struct IcmpManager {
    // Maps (guest_ip, target_ip, guest_id) -> (conn_id, real_dest_addr)
    pub(crate) flows: HashMap<(Ipv4Addr, Ipv4Addr, u16), (u64, Ipv4Addr)>,
    // Maps conn_id -> (guest_ip, target_ip, guest_id)
    pub(crate) id_to_flow: HashMap<u64, (Ipv4Addr, Ipv4Addr, u16)>,
    pub(crate) next_flow_id: u64,
}

impl Default for IcmpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IcmpManager {
    pub fn new() -> Self {
        Self {
            flows: HashMap::new(),
            id_to_flow: HashMap::new(),
            next_flow_id: 1 << 61, // Start ICMP flow IDs at 1 << 61
        }
    }

    /// Handles an incoming ICMP packet.
    pub fn handle_packet(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        packet: &ParsedPacket,
        raw_icmp_packet: &[u8],
    ) {
        let Some(NetworkPacket::Ip(IpPacket::V4(ipv4_header, _))) = &packet.network else {
            return;
        };
        let Some(TransportPacket::Icmp(icmp_header, icmp_payload)) = &packet.transport else {
            return;
        };
        if icmp_header.icmp_type == IcmpType::EchoRequest as u8 {
            let identifier = u16::from_be_bytes([icmp_header.rest[0], icmp_header.rest[1]]);
            let sequence_number = u16::from_be_bytes([icmp_header.rest[2], icmp_header.rest[3]]);
            let echo_request =
                IcmpEcho { identifier: identifier.into(), sequence_number: sequence_number.into() };
            let echo_payload = icmp_payload;
            let dest_ip = Ipv4Addr::from(ipv4_header.dest_addr);
            let guest_ip = Ipv4Addr::from(ipv4_header.source_addr);

            if dest_ip == config.host_ipv4 {
                let guest_mac = match packet.ethernet {
                    netsim_packets::EthernetPacket::Untagged { frame, .. } => frame.src_addr,
                    netsim_packets::EthernetPacket::Vlan { frame, .. } => frame.src_addr,
                };
                self.send_echo_reply(
                    responses,
                    config,
                    guest_mac,
                    ipv4_header,
                    &echo_request,
                    echo_payload,
                );
            } else {
                // External IPv4 ping! Forward to host driver via Icmp connection flow.
                let guest_id = echo_request.identifier.get();
                let flow_key = (guest_ip, dest_ip, guest_id);

                let (conn_id, _) = self.flows.entry(flow_key).or_insert_with(|| {
                    let id = self.next_flow_id;
                    self.next_flow_id += 1;
                    let conn_info = IcmpConnectionArgs {
                        destination: IpAddr::V4(dest_ip),
                        guest_ip: IpAddr::V4(guest_ip),
                        guest_id,
                    };
                    responses.push(SlirpResponse::EstablishConnection(
                        id,
                        ConnectionArgs::Icmp(conn_info),
                    ));

                    self.id_to_flow.insert(id, flow_key);
                    (id, dest_ip)
                });

                // Forward the entire raw ICMP packet (raw_icmp_packet)
                responses.push(SlirpResponse::WriteToConnection(
                    *conn_id,
                    Bytes::copy_from_slice(raw_icmp_packet),
                ));
            }
        }
    }

    /// Handles a reply from the host for an ICMP flow.
    pub fn handle_reply(
        &self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        conn_id: u64,
        icmp_reply: &[u8],
    ) {
        let Some(&(guest_ip, target_ip, _guest_id)) = self.id_to_flow.get(&conn_id) else {
            log::warn!("Received ICMP reply for unknown flow {conn_id}");
            return;
        };

        // Reconstruct the IPv4 packet containing the ICMP reply
        let icmp_len = icmp_reply.len();
        let ipv4_header_len = 20;
        let total_ipv4_len = ipv4_header_len + icmp_len;
        let mut ipv4_buffer = vec![0u8; total_ipv4_len];

        let mut ipv4_builder = Ipv4Builder::new(
            &mut ipv4_buffer,
            1, // ICMP
            target_ip,
            guest_ip,
        )
        .unwrap();

        ipv4_builder.payload_mut()[..icmp_len].copy_from_slice(icmp_reply);
        ipv4_builder.payload_len(icmp_len);
        ipv4_builder.build();

        let eth_header_len = 14;
        let total_len = eth_header_len + total_ipv4_len;
        let mut buffer = vec![0u8; total_len];

        let (eth_header_slice, eth_payload_slice) = buffer.split_at_mut(eth_header_len);
        let eth_frame = EthernetFrame::mut_from_bytes(eth_header_slice).unwrap();
        eth_frame.dst_addr = config.guest_mac;
        eth_frame.src_addr = config.gateway_mac;
        eth_frame.ethertype = 0x0800.into();
        eth_payload_slice[..total_ipv4_len].copy_from_slice(&ipv4_buffer);

        responses.push(SlirpResponse::Packet(Bytes::copy_from_slice(&buffer)));
    }

    fn send_echo_reply(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        guest_mac: MacAddr,
        original_ipv4_header: &Ipv4Header,
        echo_request: &IcmpEcho,
        echo_payload: &[u8],
    ) {
        let icmp_header_len = std::mem::size_of::<IcmpHeader>();
        let total_icmp_len = icmp_header_len + echo_payload.len();

        let ipv4_header_len = 20;
        let total_ipv4_len = ipv4_header_len + total_icmp_len;

        let mut ipv4_buffer = vec![0u8; total_ipv4_len];

        // 1. Create the builder, which gives us access to its payload slice
        let mut ipv4_builder = Ipv4Builder::new(
            &mut ipv4_buffer,
            1, // ICMP protocol number
            Ipv4Addr::from(original_ipv4_header.dest_addr),
            Ipv4Addr::from(original_ipv4_header.source_addr),
        )
        .unwrap();

        // 2. Construct the ICMP packet directly into the builder's payload
        let icmp_payload = ipv4_builder.payload_mut();
        let (icmp_header_slice, echo_payload_slice) = icmp_payload.split_at_mut(icmp_header_len);
        let icmp_header = IcmpHeader::mut_from_bytes(icmp_header_slice).unwrap();
        icmp_header.icmp_type = IcmpType::EchoReply as u8;
        icmp_header.icmp_code = 0;
        icmp_header.icmp_checksum = 0.into();

        // Write echo fields into the 4-byte rest field
        icmp_header.rest[..2].copy_from_slice(&echo_request.identifier.get().to_be_bytes());
        icmp_header.rest[2..].copy_from_slice(&echo_request.sequence_number.get().to_be_bytes());

        echo_payload_slice[..echo_payload.len()].copy_from_slice(echo_payload);

        // 3. Calculate ICMP checksum on the builder's payload
        let checksum = netsim_packets::ipv4_checksum(&icmp_payload[..total_icmp_len]);
        let icmp_header = IcmpHeader::mut_from_bytes(&mut icmp_payload[..icmp_header_len]).unwrap();
        icmp_header.icmp_checksum.set(checksum);

        // 4. Finalize the IPv4 header
        ipv4_builder.payload_len(total_icmp_len);
        ipv4_builder.build();

        // 5. Construct and send the Ethernet frame
        let eth_header_len = 14;
        let total_len = eth_header_len + total_ipv4_len;
        let mut buffer = vec![0u8; total_len];

        let (eth_header_slice, eth_payload_slice) = buffer.split_at_mut(eth_header_len);
        let eth_frame = EthernetFrame::mut_from_bytes(eth_header_slice).unwrap();
        eth_frame.dst_addr = guest_mac;
        eth_frame.src_addr = config.gateway_mac;
        eth_frame.ethertype = 0x0800.into();
        eth_payload_slice[..total_ipv4_len].copy_from_slice(&ipv4_buffer);

        responses.push(SlirpResponse::Packet(Bytes::copy_from_slice(&buffer)));
    }

    pub fn send_port_unreachable(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        offending_packet: &[u8],
    ) {
        self.send_destination_unreachable(
            responses,
            config,
            offending_packet,
            UnreachableCode::PortUnreachable,
        );
    }

    pub fn send_host_unreachable(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        offending_packet: &[u8],
    ) {
        self.send_destination_unreachable(
            responses,
            config,
            offending_packet,
            UnreachableCode::HostUnreachable,
        );
    }

    fn send_icmp_error(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        offending_packet: &[u8],
        icmp_type: IcmpType,
        icmp_code: u8,
    ) {
        let mut buffer = vec![0u8; 1500];
        let eth_header_len = 14;
        let ipv4_header_len = 20;
        let icmp_header_len = std::mem::size_of::<IcmpHeader>();

        let (eth_header_slice, eth_payload) = buffer.split_at_mut(eth_header_len);
        let eth_frame = EthernetFrame::mut_from_bytes(eth_header_slice).unwrap();
        eth_frame.dst_addr = config.guest_mac;
        eth_frame.src_addr = MacAddr { bytes: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56] };
        eth_frame.ethertype = 0x0800.into();

        let (original_ipv4_header, _) = Ipv4Header::parse(offending_packet).unwrap();

        let (ipv4_header_slice, ipv4_payload) = eth_payload.split_at_mut(ipv4_header_len);
        let mut ipv4_builder = Ipv4Builder::new(
            ipv4_header_slice,
            1,
            config.host_ipv4,
            Ipv4Addr::from(original_ipv4_header.source_addr),
        )
        .unwrap();

        let (icmp_header_slice, data_slice) = ipv4_payload.split_at_mut(icmp_header_len);
        let icmp_header = IcmpHeader::mut_from_bytes(icmp_header_slice).unwrap();
        icmp_header.icmp_type = icmp_type as u8;
        icmp_header.icmp_code = icmp_code;
        icmp_header.icmp_checksum = 0.into();
        icmp_header.rest.fill(0); // Fill the unused 4 bytes with 0

        let data_len = std::cmp::min(offending_packet.len(), 64);
        data_slice[..data_len].copy_from_slice(&offending_packet[..data_len]);

        let total_icmp_len = icmp_header_len + data_len;
        ipv4_builder.payload_len(total_icmp_len);
        ipv4_builder.build();

        // Calculate ICMP checksum
        let checksum = netsim_packets::ipv4_checksum(&ipv4_payload[..total_icmp_len]);
        let icmp_header = IcmpHeader::mut_from_bytes(&mut ipv4_payload[..icmp_header_len]).unwrap();
        icmp_header.icmp_checksum.set(checksum);

        let total_len = eth_header_len + ipv4_header_len + total_icmp_len;
        responses.push(SlirpResponse::Packet(Bytes::copy_from_slice(&buffer[..total_len])));
    }

    fn send_destination_unreachable(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        offending_packet: &[u8],
        code: UnreachableCode,
    ) {
        self.send_icmp_error(
            responses,
            config,
            offending_packet,
            IcmpType::DestUnreachable,
            code as u8,
        );
    }

    pub fn send_time_exceeded(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        offending_packet: &[u8],
    ) {
        self.send_icmp_error(responses, config, offending_packet, IcmpType::TimeExceeded, 0);
    }
}

#[cfg(test)]
#[path = "tests/icmp_tests.rs"]
mod tests;
