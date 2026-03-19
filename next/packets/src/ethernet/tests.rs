// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_arp_pcap_json() {
        let fields = &["eth.dst", "eth.src", "eth.type"];
        validate_pcap_json(
            include_bytes!("test_data/arp.pcap"),
            include_str!("test_data/arp.json"),
            fields,
        );
    }
}
