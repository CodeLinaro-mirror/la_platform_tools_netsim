#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;
    use std::path::PathBuf;

    #[test]
    fn test_llc_pcap_json() {
        let llc_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/llc/test_data");
        let fields = &["llc.dsap", "llc.ssap", "llc.control"];
        validate_pcap_json(llc_dir.join("llc_snap.pcap"), llc_dir.join("llc_snap.json"), fields);
    }
}
