// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::validate_pcap_json;
    use std::path::PathBuf;

    #[test]
    fn test_beacon_pcap_json() {
        let ieee80211_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ieee80211/test_data");
        let fields = &[
            "wlan.fc.type",
            "wlan.fc.subtype",
            "wlan.ra",
            "wlan.ta",
            "wlan.da",
            "wlan.sa",
            "wlan.bssid",
            "wlan.seq",
            "wlan.fc.frag",
        ];
        validate_pcap_json(
            ieee80211_dir.join("beacon.pcap"),
            ieee80211_dir.join("beacon.json"),
            fields,
        );
    }
}
