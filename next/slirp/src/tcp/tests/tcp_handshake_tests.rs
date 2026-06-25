// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use crate::{
    SlirpResponse,
    clock::MockClock,
    packet::{IpPacket, NetworkPacket, ParsedPacket},
    tcp::{
        manager::TcpManager,
        tests::tcp_test_utils::{CreateTcpPacketArgs, create_tcp_packet, parse_tcp_packet},
    },
    timers::TimerManager,
};

#[test]
fn test_handle_syn_sends_syn_ack() {
    let mut tcp_manager = TcpManager::new(Ipv4Addr::new(10, 0, 2, 2));
    let clock = Arc::new(Mutex::new(MockClock::new()));
    let mut timers = TimerManager::new(Box::new(clock.clone()));
    let mut responses = Vec::new();

    let guest_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)), 12345);
    let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 80);

    let ethernet_packet = create_tcp_packet(&CreateTcpPacketArgs {
        src_addr: guest_addr,
        dst_addr: host_addr,
        seq: 1000,
        ack: 0,
        syn: true,
        is_ack: false,
        fin: false,
        rst: false,
        options: &[],
        payload: &[],
    });

    let parsed_packet = ParsedPacket::parse(&ethernet_packet).unwrap();
    let (ipv4_header, tcp_packet) =
        if let Some(NetworkPacket::Ip(IpPacket::V4(h, p))) = parsed_packet.network {
            (h, p)
        } else {
            panic!();
        };

    tcp_manager.handle_packet(
        &mut responses,
        &mut timers,
        &parsed_packet,
        &ipv4_header,
        tcp_packet,
    );

    assert_eq!(responses.len(), 2);
    let response_packet = match responses.get(1).unwrap() {
        SlirpResponse::Packet(packet) => packet,
        _ => panic!("Expected a packet response"),
    };

    let (_eth, _ipv4, tcp_header, _payload) = parse_tcp_packet(response_packet);

    assert!(tcp_header.syn() && tcp_header.ack(), "Packet should be a SYN-ACK");
}
