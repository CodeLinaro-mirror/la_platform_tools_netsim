// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::net::Ipv4Addr;

use netsim_packets::tcp_checksum;

#[test]
fn test_tcp_checksum() {
    let src_addr = Ipv4Addr::new(100, 79, 15, 252);
    let dst_addr = Ipv4Addr::new(10, 0, 2, 15);
    let tcp_packet = [
        0x14, 0x51, // Source Port: 5201
        0xce, 0xfc, // Destination Port: 52732
        0x00, 0x00, 0x00, 0x01, // Sequence Number: 1
        0x00, 0x00, 0x00, 0x26, // Acknowledgement Number: 38
        0x50, 0x10, // Data Offset (5) + Flags (ACK)
        0x20, 0x00, // Window Size: 8192
        0x00, 0x00, // Checksum: 0 (to be calculated)
        0x00, 0x00, // Urgent Pointer: 0
        0x09, // Payload: one byte
    ];

    let checksum = tcp_checksum(&tcp_packet, src_addr, dst_addr);

    // This is the correct checksum value calculated manually and with online tools.
    assert_eq!(checksum, 8965);
}
