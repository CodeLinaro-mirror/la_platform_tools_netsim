// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_icmp_pcap_json() {
        let fields = &["icmp.type", "icmp.code", "ip.src", "ip.dst", "ip.proto"];
        validate_pcap_json(
            include_bytes!("test_data/icmp.pcap"),
            include_str!("test_data/icmp.json"),
            fields,
        );
    }

    #[test]
    fn test_icmpv6_pcap_json() {
        let fields = &["icmpv6.type", "icmpv6.code", "ipv6.src", "ipv6.dst", "ipv6.nxt"];
        validate_pcap_json(
            include_bytes!("test_data/icmpv6.pcap"),
            include_str!("test_data/icmpv6.json"),
            fields,
        );
    }
}
