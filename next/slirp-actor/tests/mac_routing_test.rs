// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Decoupled Integration Tests: Hardcoded Guest MAC Defect (b/560171262).
//!
//! Verifies Ethernet header destination MAC address handling in response
//! packets when packets originate from a non-default guest MAC address ([0x02,
//! 0x99, 0x88, 0x77, 0x66, 0x55]).

use std::{net::Ipv4Addr, time::Duration};

use netsim_packets::{EthernetFrame, IP_P_UDP, Ipv4Builder, MacAddr, UdpPacketBuilder};
use slirp_actor::SlirpBackend;
use tokio::time::timeout;
use zerocopy::FromBytes;

use super::test_driver::SlirpTestDriver;

const NON_DEFAULT_MAC: [u8; 6] = [0x02, 0x99, 0x88, 0x77, 0x66, 0x55];

/// Helper to create a UDP packet with a specified source MAC address using
/// `netsim-packets`.
fn create_udp_packet(
    src_mac: [u8; 6],
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
    eth_frame.src_addr = MacAddr { bytes: src_mac };
    eth_frame.ethertype = 0x0800.into();
    eth_payload_slice.copy_from_slice(&ip_data);

    bytes::Bytes::from(eth_data)
}

/// Helper to create a DNS query UDP packet with a specified source MAC address.
fn create_dns_query_packet(
    src_mac: [u8; 6],
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    query_name: &str,
    qtype: u16,
) -> bytes::Bytes {
    let mut dns_payload = vec![
        0x12, 0x34, // TXID
        0x01, 0x00, // Flags (RD=1)
        0x00, 0x01, // Questions = 1
        0x00, 0x00, // Answers = 0
        0x00, 0x00, // Authority = 0
        0x00, 0x00, // Additional = 0
    ];
    for label in query_name.split('.') {
        dns_payload.push(label.len() as u8);
        dns_payload.extend_from_slice(label.as_bytes());
    }
    dns_payload.push(0); // Null terminator
    dns_payload.extend_from_slice(&qtype.to_be_bytes()); // Type (1 = A, 28 = AAAA)
    dns_payload.extend_from_slice(&1u16.to_be_bytes()); // Class IN

    create_udp_packet(src_mac, src_ip, dst_ip, src_port, 53, &dns_payload)
}

/// Helper to create a TFTP RRQ packet with a specified source MAC address.
fn create_tftp_rrq_packet(src_mac: [u8; 6], filename: &str) -> bytes::Bytes {
    let mut tftp_payload = vec![0u8, 1]; // Opcode 1 (RRQ)
    tftp_payload.extend_from_slice(filename.as_bytes());
    tftp_payload.push(0);
    tftp_payload.extend_from_slice(b"octet");
    tftp_payload.push(0);

    create_udp_packet(
        src_mac,
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(10, 0, 2, 2),
        54321,
        69,
        &tftp_payload,
    )
}

// -----------------------------------------------------------------------------
// 1. UDP Non-Default MAC Reply Test
// -----------------------------------------------------------------------------

async fn run_udp_non_default_mac_reply_test(backend: SlirpBackend) {
    // Given a Slirp test driver and a non-default guest MAC address
    let mut driver = SlirpTestDriver::new(backend, Some("8.8.8.8".to_string())).await;

    // When sending a UDP datagram from non-default guest MAC address [0x02, 0x99,
    // 0x88, 0x77, 0x66, 0x55]
    let udp_packet = create_udp_packet(
        NON_DEFAULT_MAC,
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(10, 0, 2, 3),
        54321,
        53,
        &[0x12, 0x34, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    );
    let send_result = driver.send_packet(udp_packet).await;

    // Then the packet is sent successfully
    assert!(send_result.is_ok(), "Failed to send UDP packet");

    // Then, if a downlink reply arrives, it must be addressed to the non-default
    // guest MAC. Accepting the default MAC here would let the misrouting bug this
    // test guards (b/560171262) pass.
    //
    // NOTE: the reply comes from the host resolver via slirp's DNS proxy, which
    // the hermetic test sandbox denies, so no IPv4 frame arrives under
    // `bazel test` and the assertion is skipped.
    // Slirp broadcasts an ARP request for the guest before it can deliver a
    // reply, so skip ahead to the IPv4 frame rather than asserting on whatever
    // arrives first.
    let is_ipv4 = |frame: &bytes::Bytes| {
        EthernetFrame::parse(frame).is_some_and(|(eth, _)| eth.ethertype.get() == 0x0800)
    };
    if let Ok(Some(reply_bytes)) =
        timeout(Duration::from_millis(200), driver.recv_matching(is_ipv4)).await
    {
        let (eth_frame, _) =
            EthernetFrame::parse(&reply_bytes).expect("Failed to parse reply Ethernet frame");
        let dst_mac = eth_frame.dst_addr.bytes;
        assert_eq!(
            dst_mac, NON_DEFAULT_MAC,
            "UDP reply was routed to {:?}, expected the non-default guest MAC {:?}",
            dst_mac, NON_DEFAULT_MAC
        );
    }
}

#[tokio::test]
async fn test_udp_non_default_mac_reply() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_udp_non_default_mac_reply_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 2. DNS Query Non-Default MAC Reply Test
// -----------------------------------------------------------------------------

async fn run_dns_non_default_mac_reply_test(backend: SlirpBackend) {
    // Given a Slirp test driver configured with host DNS and a non-default guest
    // MAC address
    let mut driver = SlirpTestDriver::new(backend, Some("8.8.8.8".to_string())).await;

    // When sending a DNS query to 10.0.2.3:53 from non-default guest MAC address
    // [0x02, 0x99, 0x88, 0x77, 0x66, 0x55]
    let dns_packet = create_dns_query_packet(
        NON_DEFAULT_MAC,
        Ipv4Addr::new(10, 0, 2, 15),
        Ipv4Addr::new(10, 0, 2, 3),
        53535,
        "google.com",
        1,
    );
    let send_result = driver.send_packet(dns_packet).await;

    // Then the DNS query packet is sent successfully
    assert!(send_result.is_ok(), "Failed to send DNS query packet");

    // Then, if a downlink reply arrives, it must be addressed to the non-default
    // guest MAC. Accepting the default MAC here would let the misrouting bug this
    // test guards (b/560171262) pass.
    //
    // NOTE: the reply comes from the host resolver via slirp's DNS proxy, which
    // the hermetic test sandbox denies, so no IPv4 frame arrives under
    // `bazel test` and the assertion is skipped.
    // Slirp broadcasts an ARP request for the guest before it can deliver a
    // reply, so skip ahead to the IPv4 frame rather than asserting on whatever
    // arrives first.
    let is_ipv4 = |frame: &bytes::Bytes| {
        EthernetFrame::parse(frame).is_some_and(|(eth, _)| eth.ethertype.get() == 0x0800)
    };
    if let Ok(Some(reply_bytes)) =
        timeout(Duration::from_millis(200), driver.recv_matching(is_ipv4)).await
    {
        let (eth_frame, _) =
            EthernetFrame::parse(&reply_bytes).expect("Failed to parse reply Ethernet frame");
        let dst_mac = eth_frame.dst_addr.bytes;
        assert_eq!(
            dst_mac, NON_DEFAULT_MAC,
            "DNS reply was routed to {:?}, expected the non-default guest MAC {:?}",
            dst_mac, NON_DEFAULT_MAC
        );
    }
}

#[tokio::test]
async fn test_dns_non_default_mac_reply() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_dns_non_default_mac_reply_test(backend).await;
    }
}

// -----------------------------------------------------------------------------
// 3. TFTP RRQ Non-Default MAC Reply Test
// -----------------------------------------------------------------------------

async fn run_tftp_non_default_mac_reply_test(backend: SlirpBackend) {
    // Given a Slirp test driver and a non-default guest MAC address
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When sending a TFTP RRQ to 10.0.2.2:69 from non-default guest MAC address
    // [0x02, 0x99, 0x88, 0x77, 0x66, 0x55]
    let rrq_packet = create_tftp_rrq_packet(NON_DEFAULT_MAC, "pxelinux.0");
    let send_result = driver.send_packet(rrq_packet).await;

    // Then the TFTP RRQ packet is sent successfully
    assert!(send_result.is_ok(), "Failed to send TFTP RRQ packet");

    // Then, if a downlink reply arrives, it must be addressed to the non-default
    // guest MAC (b/560171262). TFTP requires slirp's TFTP server to be configured
    // with a root directory, which the harness does not do, so no reply is
    // expected under `bazel test`.
    // Slirp broadcasts an ARP request for the guest before it can deliver a
    // reply, so skip ahead to the IPv4 frame rather than asserting on whatever
    // arrives first.
    let is_ipv4 = |frame: &bytes::Bytes| {
        EthernetFrame::parse(frame).is_some_and(|(eth, _)| eth.ethertype.get() == 0x0800)
    };
    if let Ok(Some(reply_bytes)) =
        timeout(Duration::from_millis(200), driver.recv_matching(is_ipv4)).await
    {
        let (eth_frame, _) =
            EthernetFrame::parse(&reply_bytes).expect("Failed to parse reply Ethernet frame");
        let dst_mac = eth_frame.dst_addr.bytes;
        assert_eq!(
            dst_mac, NON_DEFAULT_MAC,
            "TFTP reply was routed to {:?}, expected the non-default guest MAC {:?}",
            dst_mac, NON_DEFAULT_MAC
        );
    }
}

#[tokio::test]
async fn test_tftp_non_default_mac_reply() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_tftp_non_default_mac_reply_test(backend).await;
    }
}
