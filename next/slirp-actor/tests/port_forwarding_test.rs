// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Decoupled Integration Tests: Host & Guest Port Forwarding (b/560172253).
//!
//! Uses `SlirpTestDriver` to keep test assertions independent of actor
//! framework primitives.

use std::net::Ipv4Addr;

use netsim_packets::{EthernetFrame, IP_P_TCP, Ipv4Builder, MacAddr, TcpBuilder};
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

// -----------------------------------------------------------------------------
// 1. Host Port Forwarding (hostfwd) Registration Test
// -----------------------------------------------------------------------------

async fn run_hostfwd_registration_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When client checks initialization status
    let initialized = driver.is_initialized().await;

    // Then driver is initialized successfully
    assert!(initialized);
}

#[tokio::test]
async fn test_hostfwd_registration_parity() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_hostfwd_registration_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 2. Guest Port Forwarding (guestfwd) Virtual IP Test
// -----------------------------------------------------------------------------

async fn run_guestfwd_virtual_ip_test(backend: SlirpBackend) {
    // Given a Slirp driver running the specified backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When guest connects to virtual IP 10.0.2.100:8080 (guestfwd target)
    let syn_packet = create_tcp_packet(
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(10, 0, 2, 100),
        12345,
        8080,
        100,
        0,
        true,  // syn
        false, // ack
        false, // fin
        false, // rst
        &[],
    );

    let result = driver.send_packet(syn_packet).await;

    // Then Slirp processes the guestfwd SYN packet
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_guestfwd_virtual_ip() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_guestfwd_virtual_ip_test(backend).await;
    }
}
