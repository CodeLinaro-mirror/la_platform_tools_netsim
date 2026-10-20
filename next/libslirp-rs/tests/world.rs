// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(windows, allow(dead_code))]

use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use bytes::Bytes;
use etherparse::{
    ArpHardwareId, ArpOperation, ArpPacket, EtherType, Ethernet2Header, Icmpv6Header, Icmpv6Type,
    Ipv6Header, LinkHeader::Ethernet2, NetHeaders, PacketBuilder, PacketHeaders, PayloadSlice,
    TransportHeader, icmpv6::NeighborAdvertisementHeader,
};
use libslirp_rs::{LibSlirp, SlirpConfig};

pub const PAYLOAD: &[u8; 23] = b"Hello, UDP echo server!";
pub const PAYLOAD_PONG: &[u8; 23] = b"Hello, UDP echo client!";

/// Unified open file descriptor count for detecting FD leaks.
///
/// NOTE: Because `/proc/self/fd` is shared across the entire process, tests
/// that assert FD leak freedom MUST be executed serially (`--test-threads=1`).
#[cfg(target_os = "linux")]
pub fn count_open_fds() -> io::Result<usize> {
    let entries = std::fs::read_dir("/proc/self/fd")?;
    Ok(entries.count())
}

#[cfg(not(target_os = "linux"))]
pub fn count_open_fds() -> io::Result<usize> {
    Ok(0)
}

/// RAII Mock UDP/DNS Server that joins its worker thread upon shutdown or drop.
pub struct MockServer {
    addr: SocketAddr,
    handle: Option<thread::JoinHandle<()>>,
}

impl MockServer {
    /// Creates a one-shot IPv4 UDP echo server.
    pub fn echo_ipv4() -> io::Result<Self> {
        Self::spawn_echo("0.0.0.0:0")
    }

    /// Creates a one-shot IPv6 UDP echo server.
    pub fn echo_ipv6() -> io::Result<Self> {
        Self::spawn_echo("[::]:0")
    }

    /// Creates a one-shot IPv4 DNS server on localhost.
    pub fn dns_ipv4() -> io::Result<Self> {
        Self::spawn_dns("127.0.0.1:0")
    }

    /// Creates a one-shot IPv6 DNS server on localhost.
    pub fn dns_ipv6() -> io::Result<Self> {
        Self::spawn_dns("[::1]:0")
    }

    fn spawn_echo(bind_addr: &str) -> io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        let addr = socket.local_addr()?;
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((len, client_addr)) = socket.recv_from(&mut buf) {
                let data = &buf[..len];
                assert_eq!(data, PAYLOAD);
                let _ = socket.send_to(PAYLOAD_PONG, client_addr);
            }
        });
        Ok(Self { addr, handle: Some(handle) })
    }

    fn spawn_dns(bind_addr: &str) -> io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        let addr = socket.local_addr()?;
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((len, client_addr)) = socket.recv_from(&mut buf) {
                assert_eq!(&buf[..len], PAYLOAD);
                let _ = socket.send_to(PAYLOAD_PONG, client_addr);
            }
        });
        Ok(Self { addr, handle: Some(handle) })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Represents a received UDP packet decoded from guest Ethernet frames.
#[derive(Clone, Debug)]
pub struct UdpReply {
    pub source_ip: IpAddr,
    pub destination_ip: IpAddr,
    pub source_port: u16,
    pub destination_port: u16,
    pub payload: Vec<u8>,
}

impl UdpReply {
    /// Asserts all reply fields against expected values with clear diagnostic
    /// messages.
    pub fn assert_matches(
        &self,
        expected_src_ip: impl Into<IpAddr>,
        expected_dst_ip: impl Into<IpAddr>,
        expected_src_port: u16,
        expected_dst_port: u16,
        expected_payload: &[u8],
    ) {
        assert_eq!(self.source_ip, expected_src_ip.into(), "source IP mismatch");
        assert_eq!(self.destination_ip, expected_dst_ip.into(), "destination IP mismatch");
        assert_eq!(self.source_port, expected_src_port, "source port mismatch");
        assert_eq!(self.destination_port, expected_dst_port, "destination port mismatch");
        assert_eq!(self.payload, expected_payload, "payload mismatch");
    }
}

/// BDD World managing test state, MockServers, LibSlirp, and step assertions.
pub struct World {
    pub slirp: Option<LibSlirp>,
    pub rx: Option<mpsc::Receiver<Bytes>>,
    pub servers: Vec<MockServer>,
    pub replies: Vec<UdpReply>,
    initial_fd_count: usize,
    next_guest_port: u16,
}

impl World {
    pub const GUEST_IPV4: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 15);
    pub const GUEST_IPV6: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x15);
    pub const VHOST_IPV4: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 2);
    pub const VHOST_IPV6: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x2);
    pub const VNAMESERVER_IPV4_1: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 3);
    pub const VNAMESERVER_IPV4_2: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 4);
    pub const VNAMESERVER_IPV6_1: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x3);
    pub const VNAMESERVER_IPV6_2: Ipv6Addr = Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0x4);
    pub const VNAMESERVER_IPV6: Ipv6Addr = Self::VNAMESERVER_IPV6_1;

    pub const GUEST_MAC: [u8; 6] = [1, 2, 3, 4, 5, 6];
    pub const VHOST_MAC: [u8; 6] = [7, 8, 9, 10, 11, 12];
    pub const DNS_PORT: u16 = 53;
    const EPHEMERAL_PORT_START: u16 = 49152;

    /// Initializes a new BDD World and captures baseline FD counts.
    pub fn new() -> Self {
        let initial_fd_count = count_open_fds().unwrap();
        Self {
            slirp: None,
            rx: None,
            servers: Vec::new(),
            replies: Vec::new(),
            initial_fd_count,
            next_guest_port: Self::EPHEMERAL_PORT_START,
        }
    }

    /// Allocates the next dynamic ephemeral guest source port.
    pub fn next_guest_port(&mut self) -> u16 {
        let port = self.next_guest_port;
        self.next_guest_port =
            self.next_guest_port.checked_add(1).unwrap_or(Self::EPHEMERAL_PORT_START);
        port
    }

    // =========================================================================
    // Given Steps (Environment & Configuration)
    // =========================================================================

    /// Given a running LibSlirp stack with default configuration.
    pub fn given_default_slirp(&mut self) {
        self.given_slirp(SlirpConfig::default());
    }

    /// Given a running LibSlirp stack with the specified configuration.
    pub fn given_slirp(&mut self, config: SlirpConfig) {
        let (tx, rx) = mpsc::channel::<Bytes>();
        let slirp = LibSlirp::new(config, Box::new(tx), None, None);
        self.slirp = Some(slirp);
        self.rx = Some(rx);
    }

    /// Given a running IPv4 UDP echo server on the host.
    pub fn given_ipv4_echo_server(&mut self) -> usize {
        let server = MockServer::echo_ipv4().expect("Failed to start IPv4 echo server");
        self.servers.push(server);
        self.servers.len() - 1
    }

    /// Given a running IPv6 UDP echo server on the host.
    pub fn given_ipv6_echo_server(&mut self) -> usize {
        let server = MockServer::echo_ipv6().expect("Failed to start IPv6 echo server");
        self.servers.push(server);
        self.servers.len() - 1
    }

    /// Given a running IPv4 DNS server on the host.
    pub fn given_ipv4_dns_server(&mut self) -> usize {
        let server = MockServer::dns_ipv4().expect("Failed to start IPv4 DNS server");
        self.servers.push(server);
        self.servers.len() - 1
    }

    /// Given a running IPv6 DNS server on the host.
    pub fn given_ipv6_dns_server(&mut self) -> usize {
        let server = MockServer::dns_ipv6().expect("Failed to start IPv6 DNS server");
        self.servers.push(server);
        self.servers.len() - 1
    }

    /// Given a LibSlirp stack configured with the specified host DNS servers.
    pub fn given_slirp_with_dns(&mut self, server_indices: &[usize]) {
        let host_dns = server_indices.iter().map(|&idx| self.server_addr(idx)).collect();
        self.given_slirp(SlirpConfig { host_dns, ..Default::default() });
    }

    /// Returns the socket address of a registered mock server.
    pub fn server_addr(&self, index: usize) -> SocketAddr {
        self.servers[index].addr()
    }

    /// Returns the port of a registered mock server.
    pub fn server_port(&self, index: usize) -> u16 {
        self.servers[index].port()
    }

    // =========================================================================
    // When Steps (Actions & Packet Transmission)
    // =========================================================================

    /// When the guest transmits an IPv4 UDP packet.
    pub fn when_send_ipv4_udp(
        &self,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        payload: &[u8],
    ) {
        let builder = PacketBuilder::ethernet2(Self::GUEST_MAC, Self::VHOST_MAC)
            .ipv4(Self::GUEST_IPV4.octets(), dst_ip.octets(), 20)
            .udp(src_port, dst_port);
        let mut pkt = Vec::with_capacity(builder.size(payload.len()));
        builder.write(&mut pkt, payload).unwrap();
        self.send_packet(pkt);
    }

    /// When the guest transmits an IPv6 UDP packet.
    pub fn when_send_ipv6_udp(
        &self,
        dst_ip: Ipv6Addr,
        src_port: u16,
        dst_port: u16,
        payload: &[u8],
    ) {
        let builder = PacketBuilder::ethernet2(Self::GUEST_MAC, Self::VHOST_MAC)
            .ipv6(Self::GUEST_IPV6.octets(), dst_ip.octets(), 20)
            .udp(src_port, dst_port);
        let mut pkt = Vec::with_capacity(builder.size(payload.len()));
        builder.write(&mut pkt, payload).unwrap();
        self.send_packet(pkt);
    }

    fn send_packet(&self, pkt: Vec<u8>) {
        if let Some(slirp) = &self.slirp {
            slirp.input(Bytes::from(pkt));
        }
    }

    /// When receiving a single UDP reply from the network stack.
    pub fn when_receive_reply(&mut self, timeout: Duration) -> &UdpReply {
        self.when_collect_replies(1, timeout);
        self.replies.first().expect("Expected at least one UDP reply")
    }

    /// When collecting multiple UDP replies from the network stack within a
    /// timeout.
    pub fn when_collect_replies(&mut self, expected_count: usize, timeout: Duration) {
        let start = Instant::now();
        while start.elapsed() < timeout && self.replies.len() < expected_count {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            let rx = match self.rx.as_ref() {
                Some(r) => r,
                None => break,
            };

            match rx.recv_timeout(remaining.min(Duration::from_millis(500))) {
                Ok(bytes) => {
                    let headers = PacketHeaders::from_ethernet_slice(&bytes).unwrap();
                    match headers.net {
                        Some(NetHeaders::Arp(arp_packet)) => {
                            self.handle_arp_request(&arp_packet);
                        }
                        Some(NetHeaders::Ipv4(ipv4_header, _)) => {
                            if let Some(TransportHeader::Udp(udp_header)) = headers.transport {
                                let payload = match headers.payload {
                                    PayloadSlice::Udp(p) => p.to_vec(),
                                    _ => Vec::new(),
                                };
                                self.replies.push(UdpReply {
                                    source_ip: IpAddr::V4(Ipv4Addr::from(ipv4_header.source)),
                                    destination_ip: IpAddr::V4(Ipv4Addr::from(
                                        ipv4_header.destination,
                                    )),
                                    source_port: udp_header.source_port,
                                    destination_port: udp_header.destination_port,
                                    payload,
                                });
                            }
                        }
                        Some(NetHeaders::Ipv6(ipv6_header, _)) => {
                            if let Some(TransportHeader::Udp(udp_header)) = headers.transport {
                                let payload = match headers.payload {
                                    PayloadSlice::Udp(p) => p.to_vec(),
                                    _ => Vec::new(),
                                };
                                self.replies.push(UdpReply {
                                    source_ip: IpAddr::V6(Ipv6Addr::from(ipv6_header.source)),
                                    destination_ip: IpAddr::V6(Ipv6Addr::from(
                                        ipv6_header.destination,
                                    )),
                                    source_port: udp_header.source_port,
                                    destination_port: udp_header.destination_port,
                                    payload,
                                });
                            } else if let (
                                Some(TransportHeader::Icmpv6(icmpv6_header)),
                                Some(Ethernet2(ether_header)),
                            ) = (headers.transport, headers.link)
                            {
                                self.handle_ndp_request(
                                    ether_header.source,
                                    &ipv6_header,
                                    &icmpv6_header,
                                    headers.payload.slice(),
                                );
                            }
                        }
                        _ => {}
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// When shutting down the LibSlirp network stack.
    pub fn when_shutdown_slirp(&mut self) {
        if let Some(slirp) = self.slirp.take() {
            slirp.shutdown();
        }
    }

    fn handle_arp_request(&self, arp_packet: &ArpPacket) {
        if arp_packet.operation == ArpOperation::REQUEST
            && arp_packet.target_protocol_addr() == Self::GUEST_IPV4.octets()
        {
            let sender_mac = arp_packet.sender_hw_addr();
            let sender_ip = arp_packet.sender_protocol_addr();
            let mut dst_mac = [0u8; 6];
            dst_mac.copy_from_slice(sender_mac);

            let eth_reply = Ethernet2Header {
                source: Self::GUEST_MAC,
                destination: dst_mac,
                ether_type: EtherType::ARP,
            };
            let arp_reply = ArpPacket::new(
                ArpHardwareId::ETHERNET,
                EtherType::IPV4,
                ArpOperation::REPLY,
                &Self::GUEST_MAC,
                &Self::GUEST_IPV4.octets(),
                sender_mac,
                sender_ip,
            )
            .unwrap();

            let mut reply_bytes = Vec::new();
            eth_reply.write(&mut reply_bytes).unwrap();
            arp_reply.write(&mut reply_bytes).unwrap();
            self.send_packet(reply_bytes);
        }
    }

    fn handle_ndp_request(
        &self,
        source_mac: [u8; 6],
        ipv6_header: &Ipv6Header,
        icmpv6_header: &Icmpv6Header,
        payload: &[u8],
    ) {
        if icmpv6_header.icmp_type == Icmpv6Type::NeighborSolicitation && payload.len() >= 16 {
            let target_ip_bytes: [u8; 16] = payload[..16].try_into().unwrap();
            let target_ip = Ipv6Addr::from(target_ip_bytes);
            if target_ip == Self::GUEST_IPV6 {
                let na_header = NeighborAdvertisementHeader {
                    router: false,
                    solicited: true,
                    r#override: true,
                };

                let builder = PacketBuilder::ethernet2(Self::GUEST_MAC, source_mac)
                    .ipv6(Self::GUEST_IPV6.octets(), ipv6_header.source, 255)
                    .icmpv6(Icmpv6Type::NeighborAdvertisement(na_header));

                let mut na_payload = Vec::new();
                na_payload.extend_from_slice(&Self::GUEST_IPV6.octets());
                na_payload.extend_from_slice(&[2, 1]);
                na_payload.extend_from_slice(&Self::GUEST_MAC);

                let mut reply_bytes = Vec::with_capacity(builder.size(na_payload.len()));
                builder.write(&mut reply_bytes, &na_payload).unwrap();
                self.send_packet(reply_bytes);
            }
        }
    }

    // =========================================================================
    // Then Steps (Assertions & Verifications)
    // =========================================================================

    /// Then the received reply matches the expected header and payload.
    pub fn then_received_reply_matches(
        &self,
        expected_src_ip: impl Into<IpAddr>,
        expected_dst_ip: impl Into<IpAddr>,
        expected_src_port: u16,
        expected_dst_port: u16,
        expected_payload: &[u8],
    ) {
        let reply = self.replies.first().expect("No UDP replies were received");
        reply.assert_matches(
            expected_src_ip,
            expected_dst_ip,
            expected_src_port,
            expected_dst_port,
            expected_payload,
        );
    }

    /// Then the received reply on the specified destination port matches.
    pub fn then_received_reply_on_port_matches(
        &self,
        dst_port: u16,
        expected_src_ip: impl Into<IpAddr>,
        expected_dst_ip: impl Into<IpAddr>,
        expected_src_port: u16,
        expected_payload: &[u8],
    ) {
        let reply = self
            .replies
            .iter()
            .find(|r| r.destination_port == dst_port)
            .unwrap_or_else(|| panic!("No reply received on destination port {dst_port}"));
        reply.assert_matches(
            expected_src_ip,
            expected_dst_ip,
            expected_src_port,
            dst_port,
            expected_payload,
        );
    }

    /// Then no UDP replies were received from the network stack.
    pub fn then_no_replies_received(&self) {
        assert!(
            self.replies.is_empty(),
            "Expected no UDP replies, but received {} reply/replies: {:?}",
            self.replies.len(),
            self.replies
        );
    }

    /// Then the packet receiver channel is disconnected upon shutdown.
    pub fn then_receiver_disconnected(&self) {
        if let Some(rx) = &self.rx {
            assert_eq!(
                rx.recv_timeout(Duration::from_millis(50)),
                Err(mpsc::RecvTimeoutError::Disconnected)
            );
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        self.when_shutdown_slirp();
        for server in &mut self.servers {
            server.join();
        }

        if !std::thread::panicking() {
            let after_fd_count = count_open_fds().unwrap();
            assert_eq!(
                self.initial_fd_count, after_fd_count,
                "File descriptor leak detected: before={}, after={}",
                self.initial_fd_count, after_fd_count
            );
        }
    }
}
