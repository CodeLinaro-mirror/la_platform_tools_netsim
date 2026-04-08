// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;

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
