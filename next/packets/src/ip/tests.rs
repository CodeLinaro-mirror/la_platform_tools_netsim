// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;
    use std::path::PathBuf;

    #[test]
    fn test_ipv4_pcap_json() {
        let icmp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/icmp/test_data");
        let fields = &["ip.src", "ip.dst", "ip.proto", "ip.ttl"];
        validate_pcap_json(icmp_dir.join("icmp.pcap"), icmp_dir.join("icmp.json"), fields);
    }

    #[test]
    fn test_ipv6_pcap_json() {
        let icmp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/icmp/test_data");
        let fields = &["ipv6.src", "ipv6.dst", "ipv6.nxt"];
        validate_pcap_json(icmp_dir.join("icmpv6.pcap"), icmp_dir.join("icmpv6.json"), fields);
    }
}
