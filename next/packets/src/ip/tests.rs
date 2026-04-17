// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_ipv4_pcap_json() {
        let fields = &["ip.src", "ip.dst", "ip.proto", "ip.ttl"];
        validate_pcap_json(
            include_bytes!("../icmp/test_data/icmp.pcap"),
            include_str!("../icmp/test_data/icmp.json"),
            fields,
        );
    }

    #[test]
    fn test_ipv6_pcap_json() {
        let fields = &["ipv6.src", "ipv6.dst", "ipv6.nxt"];
        validate_pcap_json(
            include_bytes!("../icmp/test_data/icmpv6.pcap"),
            include_str!("../icmp/test_data/icmpv6.json"),
            fields,
        );
    }
}
