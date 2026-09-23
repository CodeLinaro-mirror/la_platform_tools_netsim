// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Decoupled Integration Tests: TCP Window Scale/SACK & UDP Flow Deficiencies
//! (b/560171940, b/560171321).
//!
//! Includes BDD test coverage for:
//! 1. `test_tcp_window_scale_ignored`: TCP SYN packet containing TCP Window
//!    Scale Option (Kind 3, Length 3, Shift 7).
//! 2. `test_tcp_sack_permitted_ignored`: TCP SYN packet containing TCP SACK
//!    Permitted Option (Kind 4, Length 2).
//! 3. `test_udp_flow_churn`: Rapid opening of 1100 distinct UDP flows from
//!    sequential ephemeral ports (20000..21100).

use std::net::Ipv4Addr;

use netsim_packets::{
    EthernetFrame, IP_P_TCP, IP_P_UDP, Ipv4Builder, MacAddr, TCP_FLAG_SYN, TcpBuilder,
    UdpPacketBuilder,
};
use slirp_actor::SlirpBackend;
use zerocopy::FromBytes;

use super::test_driver::SlirpTestDriver;

/// Helper to construct a zero-copy Ethernet + IPv4 + TCP packet with options
/// using `netsim-packets`.
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
    window_size: u16,
    options: &[u8],
    payload: &[u8],
) -> bytes::Bytes {
    let mut flags = 0u16;
    if syn {
        flags |= TCP_FLAG_SYN;
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

    // The TCP data offset counts 32-bit words, so the options field has to be
    // padded to a 4-byte boundary. Using the raw length would silently drop
    // options shorter than 4 bytes (data_offset stays 5) and would overrun the
    // options slice for unaligned lengths above 4.
    let padded_options_len = options.len().div_ceil(4) * 4;
    let header_len = 20 + padded_options_len;
    let data_offset = (header_len / 4) as u8;
    let mut tcp_data = vec![0u8; header_len + payload.len()];
    let mut tcp_builder = TcpBuilder::new(&mut tcp_data, src_ip, dst_ip).unwrap();
    tcp_builder
        .source_port(src_port)
        .dest_port(dst_port)
        .sequence_num(seq)
        .ack_num(ack)
        .flags(flags)
        .window_size(window_size);

    if data_offset > 5 {
        tcp_builder.data_offset(data_offset);
        if let Some(opts) = tcp_builder.options_mut() {
            opts[..options.len()].copy_from_slice(options);
        }
    }

    if !payload.is_empty() {
        tcp_builder.payload(payload);
    }

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

/// Helper to construct a zero-copy Ethernet + IPv4 + UDP packet using
/// `netsim-packets`.
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
// 1. TCP Window Scale Option Ignored Test (b/560171940)
// -----------------------------------------------------------------------------

async fn run_tcp_window_scale_ignored_test(backend: SlirpBackend) {
    // Given a Slirp driver initialized with the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends a TCP SYN packet containing TCP Window Scale Option (Kind 3,
    // Length 3, Shift 7)
    let window_scale_option = [3u8, 3, 7, 1];
    let syn_win_scale_packet = create_tcp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(93, 184, 216, 34),
        12345,
        80,
        1000,  // seq
        0,     // ack
        true,  // syn
        false, // ack_flag
        false, // fin
        false, // rst
        8192,  // window_size
        &window_scale_option,
        &[], // payload
    );
    let result = driver.send_packet(syn_win_scale_packet).await;

    // Then Slirp handles the TCP SYN packet while driver maintains fixed 8192-byte
    // window handling
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tcp_window_scale_ignored() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_tcp_window_scale_ignored_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 2. TCP SACK Permitted Option Ignored Test (b/560171940)
// -----------------------------------------------------------------------------

async fn run_tcp_sack_permitted_ignored_test(backend: SlirpBackend) {
    // Given a Slirp driver initialized with the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest sends a TCP SYN packet containing TCP SACK Permitted Option (Kind
    // 4, Length 2)
    let sack_permitted_option = [4u8, 2, 1, 1];
    let syn_sack_packet = create_tcp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(93, 184, 216, 34),
        12345,
        80,
        1000,  // seq
        0,     // ack
        true,  // syn
        false, // ack_flag
        false, // fin
        false, // rst
        8192,  // window_size
        &sack_permitted_option,
        &[], // payload
    );
    let result = driver.send_packet(syn_sack_packet).await;

    // Then Slirp handles the TCP SYN packet with SACK Permitted option cleanly
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tcp_sack_permitted_ignored() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_tcp_sack_permitted_ignored_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 3. UDP Flow Churn Test (b/560171321)
// -----------------------------------------------------------------------------

async fn run_udp_flow_churn_test(backend: SlirpBackend) {
    // Given a Slirp driver initialized with the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest rapidly opens 1100 distinct UDP flows from sequential ephemeral
    // ports (20000..21100) to exceed MAX_UDP_FLOWS (1024) and trigger eviction
    let src_ip = Ipv4Addr::new(10, 0, 2, 15);
    let dst_ip = Ipv4Addr::new(93, 184, 216, 34);
    let dst_port = 5353;
    let payload = b"UDP flow churn datagram payload";

    for port in 20000..21100 {
        let udp_packet = create_udp_packet(src_ip, dst_ip, port, dst_port, payload);
        let result = driver.send_packet(udp_packet).await;

        // Then datagrams are sent through driver to test UDP flow tracking
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_udp_flow_churn() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_udp_flow_churn_test(backend).await;
    }
}
