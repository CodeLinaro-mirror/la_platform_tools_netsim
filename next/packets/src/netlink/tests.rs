// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_netlink_pcap_json() {
        // tshark golden file doesn't have netlink fields parsed, so we only check frame
        // metadata
        let fields = &["frame.len", "frame.cap_len"];
        validate_pcap_json(
            include_bytes!("test_data/netlink.pcap"),
            include_str!("test_data/netlink.json"),
            fields,
        );
    }
}
