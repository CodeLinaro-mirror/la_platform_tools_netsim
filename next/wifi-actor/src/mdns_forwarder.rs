// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

const MDNS_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;

struct MacAddress(u64);

impl From<MacAddress> for [u8; 6] {
    fn from(MacAddress(addr): MacAddress) -> Self {
        let [bytes @ .., _, _] = addr.to_le_bytes();
        bytes
    }
}

impl From<&[u8; 6]> for MacAddress {
    fn from(&[b0, b1, b2, b3, b4, b5]: &[u8; 6]) -> Self {
        Self(u64::from_le_bytes([b0, b1, b2, b3, b4, b5, 0, 0]))
    }
}

#[repr(C, packed)]
struct Ipv4Header {
    version_ihl: u8,
    dscp_ecn: u8,
    total_length: u16,
    identification: u16,
    flags_fragment_offset: u16,
    time_to_live: u8,
    protocol: u8,
    header_checksum: u16,
    source_ip: [u8; 4],
    destination_ip: [u8; 4],
}

impl Ipv4Header {
    fn calculate_checksum(&self) -> u16 {
        let [s0, s1, s2, s3] = self.source_ip;
        let [d0, d1, d2, d3] = self.destination_ip;
        let mut sum: u32 = [
            ((self.version_ihl as u16) << 8) | (self.dscp_ecn as u16),
            self.total_length,
            self.identification,
            self.flags_fragment_offset,
            ((self.time_to_live as u16) << 8) | (self.protocol as u16),
            self.header_checksum,
            u16::from_be_bytes([s0, s1]),
            u16::from_be_bytes([s2, s3]),
            u16::from_be_bytes([d0, d1]),
            u16::from_be_bytes([d2, d3]),
        ]
        .into_iter()
        .map(|w| w as u32)
        .sum();

        while (sum >> 16) > 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !sum as u16
    }

    fn update_checksum(&mut self) {
        self.header_checksum = 0;
        self.header_checksum = self.calculate_checksum();
    }

    fn bytes(&self) -> impl Iterator<Item = u8> {
        [self.version_ihl, self.dscp_ecn]
            .into_iter()
            .chain(self.total_length.to_be_bytes())
            .chain(self.identification.to_be_bytes())
            .chain(self.flags_fragment_offset.to_be_bytes())
            .chain([self.time_to_live, self.protocol])
            .chain(self.header_checksum.to_be_bytes())
            .chain(self.source_ip)
            .chain(self.destination_ip)
    }
}

#[repr(C, packed)]
struct UdpHeader {
    source_port: u16,
    destination_port: u16,
    length: u16,
    checksum: u16,
}

impl UdpHeader {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.source_port
            .to_be_bytes()
            .into_iter()
            .chain(self.destination_port.to_be_bytes())
            .chain(self.length.to_be_bytes())
            .chain(self.checksum.to_be_bytes())
    }
}

#[repr(C, packed)]
struct EtherHeader {
    ether_dhost: [u8; 6],
    ether_shost: [u8; 6],
    ether_type: u16,
}

const ETHER_TYPE_IP: u16 = 0x0800;

impl EtherHeader {
    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.ether_dhost.into_iter().chain(self.ether_shost).chain(self.ether_type.to_be_bytes())
    }
}

const UDP_HEADER_LEN: usize = std::mem::size_of::<UdpHeader>();
const IPV4_HEADER_LEN: usize = std::mem::size_of::<Ipv4Header>();

fn create_ethernet_frame(packet: &[u8], ip_addr: &Ipv4Addr) -> Result<Vec<u8>, String> {
    let ether_header = EtherHeader {
        ether_dhost: [0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb],
        ether_shost: [0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb],
        ether_type: ETHER_TYPE_IP,
    };

    let udp_header = UdpHeader {
        source_port: MDNS_PORT,
        destination_port: MDNS_PORT,
        length: (packet.len() + UDP_HEADER_LEN) as u16,
        checksum: 0,
    };

    let mut ipv4_header = Ipv4Header {
        version_ihl: 0x45,
        dscp_ecn: 0,
        total_length: (packet.len() + UDP_HEADER_LEN + IPV4_HEADER_LEN) as u16,
        identification: 0,
        flags_fragment_offset: 0,
        time_to_live: 64,
        protocol: 17,
        header_checksum: 0,
        source_ip: ip_addr.octets(),
        destination_ip: MDNS_IP.octets(),
    };
    ipv4_header.update_checksum();

    Ok(ether_header
        .bytes()
        .chain(ipv4_header.bytes())
        .chain(udp_header.bytes())
        .chain(packet.iter().copied())
        .collect())
}

pub async fn run_mdns_forwarder(tx: UnboundedSender<Bytes>) -> Result<(), String> {
    // Setup socket using socket2 for REUSEPORT/REUSEADDR and JoinMulticast
    let domain = socket2::Domain::IPV4;
    let socket = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
        .map_err(|e| format!("create socket failed: {:?}", e))?;

    socket.set_reuse_address(true).map_err(|e| format!("set ReuseAddr failed: {:?}", e))?;
    #[cfg(not(windows))]
    socket.set_reuse_port(true).map_err(|e| format!("set ReusePort failed: {:?}", e))?;

    socket
        .join_multicast_v4(&MDNS_IP, &Ipv4Addr::UNSPECIFIED)
        .map_err(|e| format!("join_multicast_v4 failed: {:?}", e))?;
    socket
        .set_multicast_loop_v4(false)
        .map_err(|e| format!("set_multicast_loop_v4 failed: {:?}", e))?;

    let addr: SocketAddr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), MDNS_PORT).into();
    socket.bind(&addr.into()).map_err(|e| format!("socket bind to {} failed: {:?}", &addr, e))?;

    // Convert to tokio socket
    let socket = std::net::UdpSocket::from(socket);
    let socket = tokio::net::UdpSocket::from_std(socket)
        .map_err(|e| format!("Failed to convert to tokio socket: {:?}", e))?;

    let mut buf = [0u8; 1500];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((size, src_addr)) => {
                let packet = &buf[..size];
                if let SocketAddr::V4(socket_addr_v4) = src_addr {
                    debug!("Received {} bytes from {:?}", packet.len(), socket_addr_v4);
                    match create_ethernet_frame(packet, socket_addr_v4.ip()) {
                        Ok(ethernet_frame) => {
                            if tx.send(Bytes::from(ethernet_frame)).is_err() {
                                warn!("Failed to send packet to channel (receiver closed)");
                                break;
                            }
                        }
                        Err(e) => warn!("Failed to create ethernet frame: {}", e),
                    }
                } else {
                    warn!("Forwarding mDNS from IPv6 is not supported: {:?}", src_addr);
                }
            }
            Err(e) => {
                warn!("recv_from failed: {:?}", e);
            }
        }
    }
    Ok(())
}
