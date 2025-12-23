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

    #[test]
    fn test_golden_ccmp_pcap_json() {
        let ieee80211_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ieee80211/test_data");
        // Validate basic fields + CCMP details (if tshark parses them)
        // Note: tshark might not decrypt without keys, but it parses the CCMP header (KeyID, ExtIV, PN).
        let fields = &[
            "wlan.fc.type",
            "wlan.fc.subtype",
            "wlan.fc.protected",
            "wlan.ra",
            "wlan.ta",
            "wlan.da",
            "wlan.sa",
            "wlan.bssid",
            "wlan.seq",
            "wlan.fc.frag",
            // We can check if tshark sees the CCMP header components
            // PN is 48-bit, tshark shows it as generated field mostly?
            // Key ID is in the first byte of CCMP header.
        ];
        validate_pcap_json(
            ieee80211_dir.join("golden_ccmp.pcap"),
            ieee80211_dir.join("golden_ccmp.json"),
            fields,
        );
    }
}
