// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;
    use std::path::PathBuf;

    #[test]
    fn test_arp_pcap_json() {
        let eth_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ethernet/test_data");
        let fields = &["eth.dst", "eth.src", "eth.type"];
        validate_pcap_json(eth_dir.join("arp.pcap"), eth_dir.join("arp.json"), fields);
    }
}
