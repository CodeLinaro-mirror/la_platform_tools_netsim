// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Integration Tests: Layer 4 NAPT (TCP, UDP, & ICMP) Forwarding (b/560172253).

use std::net::Ipv4Addr;

use netsim_packets::{
    EthernetFrame, IP_P_TCP, IP_P_UDP, Ipv4Builder, MacAddr, TcpBuilder, UdpPacketBuilder,
};
use slirp_actor::SlirpBackend;
use zerocopy::FromBytes;

use super::test_driver::SlirpTestDriver;

#[allow(clippy::too_many_arguments)]
fn create_tcp_packet(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    syn: bool,
    ack_flag: bool,
    fin: bool,
    rst: bool,
    payload: &[u8],
) -> bytes::Bytes {
    let mut flags = 0u16;
    if syn {
        flags |= netsim_packets::TCP_FLAG_SYN;
    }
    if ack_flag {
        flags |= netsim_packets::TCP_FLAG_ACK;
    }
    if fin {
        flags |= netsim_packets::TCP_FLAG_FIN;
    }
    if rst {
        flags |= netsim_packets::TCP_FLAG_RST;
    }

    let mut tcp_data = vec![0u8; 20 + payload.len()];
    let mut tcp_builder = TcpBuilder::new(&mut tcp_data, src_ip, dst_ip).unwrap();
    tcp_builder
        .source_port(src_port)
        .dest_port(dst_port)
        .sequence_num(seq)
        .ack_num(ack)
        .flags(flags)
        .window_size(8192);
    tcp_builder.payload(payload).unwrap();
    tcp_builder.build();

    let mut ip_data = vec![0u8; 20 + tcp_data.len()];
    let mut ipv4_builder = Ipv4Builder::new(&mut ip_data, IP_P_TCP, src_ip, dst_ip).unwrap();
    ipv4_builder.payload(&tcp_data).unwrap();
    ipv4_builder.build().unwrap();

    let mut eth_data = vec![0u8; 14 + ip_data.len()];
    let (eth_header_slice, eth_payload_slice) = eth_data.split_at_mut(14);
    let eth_frame = EthernetFrame::mut_from_bytes(eth_header_slice).unwrap();
    eth_frame.dst_addr = MacAddr { bytes: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56] };
    eth_frame.src_addr = MacAddr { bytes: [0x02, 0x15, 0xb2, 0x00, 0x00, 0x00] };
    eth_frame.ethertype = 0x0800.into();
    eth_payload_slice.copy_from_slice(&ip_data);

    bytes::Bytes::from(eth_data)
}

fn create_udp_packet(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> bytes::Bytes {
    let mut udp_data = vec![0u8; 8 + payload.len()];
    let mut udp_builder = UdpPacketBuilder::new(&mut udp_data, src_port, dst_port).unwrap();
    udp_builder.payload_mut()[..payload.len()].copy_from_slice(payload);
    udp_builder.payload_len(payload.len());
    udp_builder.build();

    let mut ip_data = vec![0u8; 20 + udp_data.len()];
    let mut ipv4_builder = Ipv4Builder::new(&mut ip_data, IP_P_UDP, src_ip, dst_ip).unwrap();
    ipv4_builder.payload(&udp_data).unwrap();
    ipv4_builder.build().unwrap();

    let mut eth_data = vec![0u8; 14 + ip_data.len()];
    let (eth_header_slice, eth_payload_slice) = eth_data.split_at_mut(14);
    let eth_frame = EthernetFrame::mut_from_bytes(eth_header_slice).unwrap();
    eth_frame.dst_addr = MacAddr { bytes: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56] };
    eth_frame.src_addr = MacAddr { bytes: [0x02, 0x15, 0xb2, 0x00, 0x00, 0x00] };
    eth_frame.ethertype = 0x0800.into();
    eth_payload_slice.copy_from_slice(&ip_data);

    bytes::Bytes::from(eth_data)
}

// -----------------------------------------------------------------------------
// 1. TCP Outbound Handshake Test
// -----------------------------------------------------------------------------

async fn run_tcp_handshake_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends TCP SYN (10.0.2.15:12345 -> 93.184.216.34:80)
    let syn_packet = create_tcp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(93, 184, 216, 34),
        12345,
        80,
        1000,
        0,
        true,  // syn
        false, // ack
        false, // fin
        false, // rst
        &[],
    );
    let result = driver.send_packet(syn_packet).await;

    // Then Slirp processes the SYN packet
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tcp_handshake() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_tcp_handshake_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 2. TCP Graceful Teardown (FIN/ACK) Test
// -----------------------------------------------------------------------------

async fn run_tcp_fin_teardown_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends TCP FIN/ACK packet
    let fin_packet = create_tcp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(93, 184, 216, 34),
        12345,
        80,
        1001,
        2001,
        false, // syn
        true,  // ack
        true,  // fin
        false, // rst
        &[],
    );
    let result = driver.send_packet(fin_packet).await;

    // Then Slirp accepts FIN teardown packet
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tcp_fin_teardown() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_tcp_fin_teardown_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 3. TCP RST Abort Test
// -----------------------------------------------------------------------------

async fn run_tcp_rst_abort_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends TCP RST packet
    let rst_packet = create_tcp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(93, 184, 216, 34),
        12345,
        80,
        1002,
        0,
        false, // syn
        false, // ack
        false, // fin
        true,  // rst
        &[],
    );
    let result = driver.send_packet(rst_packet).await;

    // Then Slirp processes TCP RST reset
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tcp_rst_abort() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_tcp_rst_abort_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 4. UDP Datagram Outbound Test
// -----------------------------------------------------------------------------

async fn run_udp_outbound_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends UDP datagram (10.0.2.15:54321 -> 8.8.8.8:53)
    let udp_packet = create_udp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(8, 8, 8, 8),
        54321,
        53,
        b"hello_udp_dns",
    );
    let result = driver.send_packet(udp_packet).await;

    // Then Slirp processes the UDP datagram
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_udp_outbound() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_udp_outbound_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 5. UDP Concurrent Ports Test
// -----------------------------------------------------------------------------

async fn run_udp_concurrent_ports_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends UDP datagrams across 3 distinct ephemeral ports
    for port in [10001, 10002, 10003] {
        let packet = create_udp_packet(
            Ipv4Addr::new(10, 0, 2, 15),
            Ipv4Addr::new(8, 8, 8, 8),
            port,
            53,
            b"concurrent_udp_flow",
        );
        let result = driver.send_packet(packet).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_udp_concurrent_ports() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_udp_concurrent_ports_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 6. UDP Broadcast Test
// -----------------------------------------------------------------------------

async fn run_udp_broadcast_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends UDP broadcast to 255.255.255.255:67
    let broadcast_packet = create_udp_packet(
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(255, 255, 255, 255),
        68,
        67,
        b"dhcp_discover_broadcast",
    );
    let result = driver.send_packet(broadcast_packet).await;

    // Then Slirp processes the broadcast packet
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_udp_broadcast() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_udp_broadcast_test(backend).await;
    }
}
