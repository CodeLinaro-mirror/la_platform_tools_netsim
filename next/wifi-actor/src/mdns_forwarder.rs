// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

const MDNS_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;

struct MacAddress(u64);

impl MacAddress {
    fn to_be_bytes(&self) -> [u8; 6] {
        self.0.to_le_bytes()[0..6].try_into().unwrap()
    }
}

impl From<MacAddress> for [u8; 6] {
    fn from(MacAddress(addr): MacAddress) -> Self {
        let bytes = u64::to_le_bytes(addr);
        bytes[0..6].try_into().unwrap()
    }
}

impl From<&[u8; 6]> for MacAddress {
    fn from(bytes: &[u8; 6]) -> Self {
        Self(u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], 0, 0]))
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

macro_rules! be_vec {
    ( $( $x:expr ),* ) => {
         Vec::<u8>::new().iter().copied()
         $( .chain($x.to_be_bytes()) )*
         .collect()
       };
    }

impl Ipv4Header {
    fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;
        let fixed_bytes: [u8; 20] = self.to_be_bytes();
        for i in 0..10 {
            let word = ((fixed_bytes[i * 2] as u16) << 8) | (fixed_bytes[i * 2 + 1] as u16);
            sum += word as u32;
        }
        while (sum >> 16) > 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !sum as u16
    }

    fn update_checksum(&mut self) {
        self.header_checksum = 0;
        self.header_checksum = self.calculate_checksum();
    }

    fn to_be_bytes(&self) -> [u8; 20] {
        let mut v: Vec<u8> = be_vec![
            self.version_ihl,
            self.dscp_ecn,
            self.total_length,
            self.identification,
            self.flags_fragment_offset,
            self.time_to_live,
            self.protocol,
            self.header_checksum
        ];
        v.extend(Ipv4Addr::from(self.source_ip).octets());
        v.extend(Ipv4Addr::from(self.destination_ip).octets());
        v.try_into().unwrap()
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
    fn to_be_bytes(&self) -> [u8; 8] {
        let v: Vec<u8> =
            be_vec![self.source_port, self.destination_port, self.length, self.checksum];
        v.try_into().unwrap()
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
    fn to_be_bytes(&self) -> [u8; 14] {
        let v: Vec<u8> = be_vec![
            MacAddress::from(&self.ether_dhost),
            MacAddress::from(&self.ether_shost),
            self.ether_type
        ];
        v.try_into().unwrap()
    }
}

const UDP_HEADER_LEN: usize = std::mem::size_of::<UdpHeader>();
const IPV4_HEADER_LEN: usize = std::mem::size_of::<Ipv4Header>();
const ETHER_HEADER_LEN: usize = std::mem::size_of::<EtherHeader>();

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

    let mut response_packet =
        Vec::with_capacity(ETHER_HEADER_LEN + IPV4_HEADER_LEN + UDP_HEADER_LEN + packet.len());
    response_packet.extend_from_slice(&ether_header.to_be_bytes());
    response_packet.extend_from_slice(&ipv4_header.to_be_bytes());
    response_packet.extend_from_slice(&udp_header.to_be_bytes());
    response_packet.extend_from_slice(packet);

    Ok(response_packet)
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
