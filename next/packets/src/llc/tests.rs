#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_llc_pcap_json() {
        let fields = &["llc.dsap", "llc.ssap", "llc.control"];
        validate_pcap_json(
            include_bytes!("test_data/llc_snap.pcap"),
            include_str!("test_data/llc_snap.json"),
            fields,
        );
    }
}
