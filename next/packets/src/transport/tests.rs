// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_tcp_pcap_json() {
        let transport_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/transport/test_data");
        let fields = &["tcp.srcport", "tcp.dstport"];
        validate_pcap_json(transport_dir.join("tcp.pcap"), transport_dir.join("tcp.json"), fields);
    }

    #[test]
    fn test_udp_pcap_json() {
        let transport_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/transport/test_data");
        let fields = &["udp.srcport", "udp.dstport"];
        validate_pcap_json(transport_dir.join("udp.pcap"), transport_dir.join("udp.json"), fields);
    }
}
