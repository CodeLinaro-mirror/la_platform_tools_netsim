// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use crate::{
        transport::{TcpBuilder, UdpBuilder},
        utils::test_utils::validate_pcap_json,
    };

    #[test]
    fn test_transport_builders_fail_on_small_buffer() {
        let mut small_buffer = [0u8; 5];

        assert!(
            TcpBuilder::new(
                &mut small_buffer,
                Ipv4Addr::new(127, 0, 0, 1),
                Ipv4Addr::new(127, 0, 0, 2)
            )
            .is_none()
        );

        assert!(
            UdpBuilder::new(
                &mut small_buffer,
                Ipv4Addr::new(127, 0, 0, 1),
                Ipv4Addr::new(127, 0, 0, 2),
                1234,
                5678
            )
            .is_none()
        );
    }

    #[test]
    fn test_tcp_builder_payload_len() {
        let mut buffer = [0u8; 100];
        let mut builder =
            TcpBuilder::new(&mut buffer, Ipv4Addr::new(127, 0, 0, 1), Ipv4Addr::new(127, 0, 0, 2))
                .unwrap();

        // Use payload_mut to write some data
        let payload = builder.payload_mut().unwrap();
        payload[0] = 1;
        payload[1] = 2;

        // Manually set payload_len
        builder.payload_len(2);

        let total_len = builder.build().unwrap();
        assert_eq!(total_len, 20 + 2); // 20 bytes TCP header + 2 bytes payload
    }

    #[test]
    fn test_udp_builder_len_and_build() {
        let mut buffer = [0u8; 100];
        let mut builder = UdpBuilder::new(
            &mut buffer,
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(127, 0, 0, 2),
            1234,
            5678,
        )
        .unwrap();

        // Write payload
        let payload = builder.payload_mut();
        payload[0] = 1;
        payload[1] = 2;
        payload[2] = 3;

        builder.payload_len(3);

        let total_len = builder.build().unwrap();
        assert_eq!(total_len, 8 + 3); // 8 bytes UDP header + 3 bytes payload

        // Verify checksum calculation (0xE2D2 in big endian)
        assert_eq!(u16::from_be_bytes([buffer[6], buffer[7]]), 0xE2D2);
    }

    #[test]
    fn test_tcp_builder_overflow() {
        let mut buffer = [0u8; 30]; // 20 bytes header, only 10 bytes remaining
        let mut builder =
            TcpBuilder::new(&mut buffer, Ipv4Addr::new(127, 0, 0, 1), Ipv4Addr::new(127, 0, 0, 2))
                .unwrap();

        // payload_mut should work
        assert!(builder.payload_mut().is_some());

        // payload of 10 bytes should work
        assert!(builder.payload(&[0u8; 10]).is_some());

        // payload of 11 bytes should fail
        assert!(builder.payload(&[0u8; 11]).is_none());
    }

    #[test]
    fn test_udp_builder_overflow() {
        let mut buffer = [0u8; 15]; // 8 bytes header, only 7 bytes remaining
        let mut builder = UdpBuilder::new(
            &mut buffer,
            Ipv4Addr::new(127, 0, 0, 1),
            Ipv4Addr::new(127, 0, 0, 2),
            1234,
            5678,
        )
        .unwrap();

        // payload of 7 bytes should work
        assert!(builder.payload(&[0u8; 7]).is_some());

        // payload of 8 bytes should fail
        assert!(builder.payload(&[0u8; 8]).is_none());
    }

    #[test]
    fn test_tcp_pcap_json() {
        let fields = &["tcp.srcport", "tcp.dstport"];
        validate_pcap_json(
            include_bytes!("test_data/tcp.pcap"),
            include_str!("test_data/tcp.json"),
            fields,
        );
    }

    #[test]
    fn test_udp_pcap_json() {
        let fields = &["udp.srcport", "udp.dstport"];
        validate_pcap_json(
            include_bytes!("test_data/udp.pcap"),
            include_str!("test_data/udp.json"),
            fields,
        );
    }
}
