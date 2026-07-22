// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Stateless DHCPv6 protocol implementation.

use std::net::Ipv6Addr;

use bytes::Bytes;
use netsim_packets::{
    DHCPV6_CLIENT_PORT, DHCPV6_SERVER_PORT, Dhcpv6Header, Dhcpv6OptionHeader, Dhcpv6OptionIterator,
    EthernetFrame, EthernetPacket, Ipv6Builder, MSG_INFORMATION_REQUEST, MSG_REPLY, MacAddr,
    OPTION_CLIENTID, OPTION_DNS_SERVERS, OPTION_DOMAIN_LIST, OPTION_SERVERID, UdpHeader,
    UdpPacketBuilder,
};
use zerocopy::{FromBytes, U16};

use crate::{Config, SlirpResponse, dns::encode_dns_search_list, packet::ParsedPacket};

pub struct Dhcpv6Manager;

impl Default for Dhcpv6Manager {
    fn default() -> Self {
        Self::new()
    }
}

impl Dhcpv6Manager {
    pub fn new() -> Self {
        Self
    }

    /// Handles an incoming DHCPv6 packet.
    pub fn handle_packet(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        packet: &ParsedPacket,
        guest_ip: Ipv6Addr,
        _udp_header: &UdpHeader,
        udp_payload: &[u8],
    ) {
        let Some((dhcpv6_hdr, options_payload)) = Dhcpv6Header::parse(udp_payload) else {
            return;
        };

        if dhcpv6_hdr.msg_type == MSG_INFORMATION_REQUEST {
            // Parse options to find Client Identifier
            let mut client_id = None;
            let iter = Dhcpv6OptionIterator::new(options_payload);
            for (code, val) in iter {
                if code == OPTION_CLIENTID {
                    client_id = Some(val);
                    break;
                }
            }

            // Client ID is required in stateless DHCPv6 Information-Request to send a Reply
            if let Some(client_id_val) = client_id {
                let guest_mac = match packet.ethernet {
                    EthernetPacket::Untagged { frame, .. } => frame.src_addr,
                    EthernetPacket::Vlan { frame, .. } => frame.src_addr,
                };
                self.send_reply(
                    responses,
                    config,
                    guest_mac,
                    guest_ip,
                    dhcpv6_hdr.transaction_id,
                    client_id_val,
                );
            }
        }
    }

    fn send_reply(
        &mut self,
        responses: &mut Vec<SlirpResponse>,
        config: &Config,
        guest_mac: MacAddr,
        dest_ip: Ipv6Addr,
        transaction_id: [u8; 3],
        client_id: &[u8],
    ) {
        // Calculate options size:
        // 1. Client Identifier Option: 4 (header) + client_id.len()
        let client_id_opt_len = 4 + client_id.len();

        // 2. Server Identifier Option (DUID-LL based on gateway MAC): DUID-LL is option
        //    code 2, DUID type 3, HW type 1, MAC (6 bytes) Total option len = 4
        //    (header) + 2 (duid type) + 2 (hw type) + 6 (mac) = 14 bytes
        let server_id_opt_len = 14;

        // 3. DNS Servers Option (Option 23): 4 (header) + 16 (IPv6) = 20 bytes
        let dns_servers_opt_len = 20;

        // 4. Domain Search List Option (Option 24):
        let domain_list_bytes =
            config.dns_search.as_ref().map(|search| encode_dns_search_list(search));
        let domain_list_opt_len =
            domain_list_bytes.as_ref().map(|bytes| 4 + bytes.len()).unwrap_or(0);

        let total_options_len =
            client_id_opt_len + server_id_opt_len + dns_servers_opt_len + domain_list_opt_len;

        let dhcpv6_header_len = 4;
        let total_dhcpv6_len = dhcpv6_header_len + total_options_len;
        let udp_header_len = 8;
        let total_udp_len = udp_header_len + total_dhcpv6_len;
        let ipv6_header_len = 40;
        let total_ipv6_len = ipv6_header_len + total_udp_len;
        let eth_header_len = 14;
        let total_len = eth_header_len + total_ipv6_len;

        let mut buffer = vec![0u8; total_len];

        let src_ip = config.host_ipv6;

        // 1. Fill Ethernet Header
        let (eth_slice, eth_payload) = buffer.split_at_mut(eth_header_len);
        let eth_frame = EthernetFrame::mut_from_bytes(eth_slice).unwrap();
        eth_frame.dst_addr = guest_mac;
        eth_frame.src_addr = config.gateway_mac;
        eth_frame.ethertype = 0x86DD.into(); // IPv6

        // 2. Build IPv6 Header
        let (ipv6_slice, ipv6_payload) = eth_payload.split_at_mut(ipv6_header_len);
        let mut ipv6_builder =
            Ipv6Builder::new(ipv6_slice, netsim_packets::IP_P_UDP, src_ip, dest_ip).unwrap();
        ipv6_builder.hop_limit(64);

        // 3. Build UDP Header
        let (udp_slice, udp_payload) = ipv6_payload.split_at_mut(udp_header_len);
        let mut udp_builder =
            UdpPacketBuilder::new(udp_slice, DHCPV6_SERVER_PORT, DHCPV6_CLIENT_PORT).unwrap();

        // 4. Build DHCPv6 Header
        let (dhcpv6_slice, options_slice) = udp_payload.split_at_mut(dhcpv6_header_len);
        let dhcpv6_hdr = Dhcpv6Header::mut_from_bytes(dhcpv6_slice).unwrap();
        dhcpv6_hdr.msg_type = MSG_REPLY;
        dhcpv6_hdr.transaction_id = transaction_id;

        // 5. Fill Options
        let mut offset = 0;

        // 5.1. Server Identifier (DUID-LL based on gateway MAC)
        let srv_id_hdr =
            Dhcpv6OptionHeader::mut_from_bytes(&mut options_slice[offset..offset + 4]).unwrap();
        srv_id_hdr.code = U16::new(OPTION_SERVERID);
        srv_id_hdr.len = U16::new(10); // DUID type (2) + HW type (2) + MAC (6) = 10 bytes
        offset += 4;
        options_slice[offset..offset + 2].copy_from_slice(&3u16.to_be_bytes());
        options_slice[offset + 2..offset + 4].copy_from_slice(&1u16.to_be_bytes());
        options_slice[offset + 4..offset + 10].copy_from_slice(&config.gateway_mac.bytes);
        offset += 10;

        // 5.2. Client Identifier (copy from request)
        let cli_id_hdr =
            Dhcpv6OptionHeader::mut_from_bytes(&mut options_slice[offset..offset + 4]).unwrap();
        cli_id_hdr.code = U16::new(OPTION_CLIENTID);
        cli_id_hdr.len = U16::new(client_id.len() as u16);
        offset += 4;
        options_slice[offset..offset + client_id.len()].copy_from_slice(client_id);
        offset += client_id.len();

        // 5.3. DNS Servers Option
        let dns_hdr =
            Dhcpv6OptionHeader::mut_from_bytes(&mut options_slice[offset..offset + 4]).unwrap();
        dns_hdr.code = U16::new(OPTION_DNS_SERVERS);
        dns_hdr.len = U16::new(16); // 1 IPv6 address = 16 bytes
        offset += 4;
        options_slice[offset..offset + 16].copy_from_slice(&config.host_ipv6.octets());
        offset += 16;

        // 5.4. Domain Search List Option (optional)
        if let Some(bytes) = domain_list_bytes {
            let dom_hdr =
                Dhcpv6OptionHeader::mut_from_bytes(&mut options_slice[offset..offset + 4]).unwrap();
            dom_hdr.code = U16::new(OPTION_DOMAIN_LIST);
            dom_hdr.len = U16::new(bytes.len() as u16);
            offset += 4;
            options_slice[offset..offset + bytes.len()].copy_from_slice(&bytes);
        }

        // 6. Finalize UDP and IPv6
        udp_builder.payload_len(total_dhcpv6_len);
        udp_builder.build();

        ipv6_builder.payload_len(total_udp_len);
        ipv6_builder.build();

        responses.push(SlirpResponse::Packet(Bytes::copy_from_slice(&buffer)));
    }
}
