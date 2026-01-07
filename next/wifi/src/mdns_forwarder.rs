// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::{WifiError, WifiResult};
use bytes::Bytes;
use log::{debug, warn};
use netsim_packets::ethernet::{ether_type, EthernetFrame, MacAddr};
use netsim_packets::ip::frame::{Ipv4Header, IP_P_UDP};
use netsim_packets::transport::udp::UdpHeader;
use socket2::{Protocol, Socket};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::mpsc;
use zerocopy::{IntoBytes, U16};

const MDNS_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;

// Protocol numbers
const IP_P_UDP_U8: u8 = IP_P_UDP;

// Define constants for header sizes (bytes)
const UDP_HEADER_LEN: usize = std::mem::size_of::<UdpHeader>();
const IPV4_HEADER_LEN: usize = std::mem::size_of::<Ipv4Header>();
const ETHER_HEADER_LEN: usize = std::mem::size_of::<EthernetFrame>();

/// Creates a new UDP socket to bind to `port` with REUSEPORT option.
/// `non_block` indicates whether to set O_NONBLOCK for the socket.
fn new_socket(addr: SocketAddr, non_block: bool) -> WifiResult<Socket> {
    let domain = match addr {
        SocketAddr::V4(_) => socket2::Domain::IPV4,
        SocketAddr::V6(_) => socket2::Domain::IPV6,
    };

    let socket = Socket::new(domain, socket2::Type::DGRAM, Some(Protocol::UDP))
        .map_err(|e| WifiError::Network(format!("create socket failed: {:?}", e)))?;

    socket
        .set_reuse_address(true)
        .map_err(|e| WifiError::Network(format!("set ReuseAddr failed: {:?}", e)))?;
    #[cfg(not(windows))]
    socket.set_reuse_port(true)?;

    #[cfg(unix)] // this is currently restricted to Unix's in socket2
    socket
        .set_reuse_port(true)
        .map_err(|e| WifiError::Network(format!("set ReusePort failed: {:?}", e)))?;

    if non_block {
        socket
            .set_nonblocking(true)
            .map_err(|e| WifiError::Network(format!("set O_NONBLOCK: {:?}", e)))?;
    }

    socket
        .join_multicast_v4(&MDNS_IP, &Ipv4Addr::UNSPECIFIED)
        .map_err(|e| WifiError::Network(format!("join_multicast_v4 failed: {:?}", e)))?;
    socket.set_multicast_loop_v4(false).expect("set_multicast_loop_v4 call failed");

    socket
        .bind(&addr.into())
        .map_err(|e| WifiError::Network(format!("socket bind to {} failed: {:?}", &addr, e)))?;

    Ok(socket)
}

fn calculate_ipv4_checksum(header: &mut Ipv4Header) {
    header.header_checksum = U16::new(0); // Reset checksum
    let bytes = header.as_bytes();
    let mut sum: u32 = 0;
    // Process 16-bit words
    // Ipv4 header is multiple of 4 bytes (20 bytes), so it's even.
    for i in 0..bytes.len() / 2 {
        let word = ((bytes[i * 2] as u16) << 8) | (bytes[i * 2 + 1] as u16);
        sum += word as u32;
    }

    // Handle carries
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // One's complement
    header.header_checksum = U16::new(!sum as u16);
}

fn create_ethernet_frame(packet: &[u8], ip_addr: &Ipv4Addr) -> WifiResult<Vec<u8>> {
    let ether_header = EthernetFrame {
        // mDNS multicast IP address
        dst_addr: MacAddr::new([0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb]),
        src_addr: MacAddr::new([0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb]),
        ethertype: U16::new(ether_type::IPV4),
    };

    // Create UDP Header
    let udp_header = UdpHeader {
        source_port: U16::new(MDNS_PORT),
        dest_port: U16::new(MDNS_PORT),
        length: U16::new((packet.len() + UDP_HEADER_LEN) as u16),
        // Usually 0 for mDNS
        checksum: U16::new(0),
    };

    // Create IPv4 Header
    let mut ipv4_header = Ipv4Header {
        version_ihl: 0x45, // Version 4, IHL 5
        dscp_ecn: 0,
        total_length: U16::new((packet.len() + UDP_HEADER_LEN + IPV4_HEADER_LEN) as u16),
        identification: U16::new(0),
        flags_fragment_offset: U16::new(0),
        ttl: 64,
        protocol: IP_P_UDP_U8,
        header_checksum: U16::new(0),
        source_addr: ip_addr.octets(),
        // mDNS multicast
        dest_addr: MDNS_IP.octets(),
    };
    calculate_ipv4_checksum(&mut ipv4_header);

    // Combine Headers and Payload (Safely using Vec)
    let mut response_packet =
        Vec::with_capacity(ETHER_HEADER_LEN + IPV4_HEADER_LEN + UDP_HEADER_LEN + packet.len());
    response_packet.extend_from_slice(ether_header.as_bytes());
    response_packet.extend_from_slice(ipv4_header.as_bytes());
    response_packet.extend_from_slice(udp_header.as_bytes());
    response_packet.extend_from_slice(packet);

    Ok(response_packet)
}

pub fn run_mdns_forwarder(tx: mpsc::Sender<Bytes>) -> WifiResult<()> {
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), MDNS_PORT);
    let socket = new_socket(addr.into(), false)?;

    // Typical max mDNS packet size
    let mut buf: [MaybeUninit<u8>; 1500] = [MaybeUninit::new(0_u8); 1500];
    loop {
        let (size, src_addr) = socket
            .recv_from(&mut buf[..])
            .map_err(|e| WifiError::Network(format!("recv_from failed: {:?}", e)))?;
        // SAFETY: `recv_from` implementation promises not to write uninitialized bytes to `buf`.
        // Documentation: https://docs.rs/socket2/latest/socket2/struct.Socket.html#method.recv_from
        let packet = unsafe { &*(&buf[..size] as *const [MaybeUninit<u8>] as *const [u8]) };
        if let Some(socket_addr_v4) = src_addr.as_socket_ipv4() {
            debug!("Received {} bytes from {:?}", packet.len(), socket_addr_v4);
            match create_ethernet_frame(packet, socket_addr_v4.ip()) {
                Ok(ethernet_frame) => {
                    if let Err(e) = tx.send(ethernet_frame.into()) {
                        warn!("Failed to send packet: {}", e);
                    }
                }
                Err(e) => warn!("Failed to create ethernet frame from UDP payload: {}", e),
            };
        } else {
            warn!("Forwarding mDNS from IPv6 is not supported: {:?}", src_addr);
        }
    }
}
