// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    // use crate::utils::test_utils::validate_pcap_json;
    use std::path::PathBuf;

    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_netlink_pcap_json() {
        let netlink_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/netlink/test_data");
        // tshark golden file doesn't have netlink fields parsed, so we only check frame
        // metadata
        let fields = &["frame.len", "frame.cap_len"];
        validate_pcap_json(
            netlink_dir.join("netlink.pcap"),
            netlink_dir.join("netlink.json"),
            fields,
        );
    }
}
