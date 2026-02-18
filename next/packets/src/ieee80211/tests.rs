// Copyright 2025 The Android Open Source Project

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::utils::test_utils::validate_pcap_json;

    #[test]
    fn test_beacon_pcap_json() {
        // Validation with tshark output requires file paths.
        // We assume validate_pcap_json works as it is an existing pattern.

        // For test_beacon_pcap_json, if it uses CARGO_MANIFEST_DIR, it relies on "data"
        // attr. I will just fix test_beacon_details to use include_bytes!

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
        // Note: tshark might not decrypt without keys, but it parses the CCMP header
        // (KeyID, ExtIV, PN).
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

    #[test]
    fn test_beacon_details() {
        let pcap_bytes = include_bytes!("test_data/beacon.pcap");

        // Skip pcap header (24 bytes) + pcap record header (16 bytes)
        // This is fragile but sufficient for this specific test file if we know it has
        // one packet.

        let mut reader =
            crate::pcap::PcapReader::new(std::io::Cursor::new(pcap_bytes)).expect("PcapReader");
        let (_header, data) = reader.next_record().expect("next_record").expect("some record");

        // This is 802.11 frame (raw)
        // Parse it using Ieee80211
        use crate::ieee80211::BeaconFrameHeader;

        // Beacon Header is 24 bytes
        let hdr = zerocopy::Ref::<&[u8], BeaconFrameHeader>::from_bytes(&data[..24])
            .expect("BeaconHeader");

        assert_eq!(hdr.frame_control.frame_type(), crate::ieee80211::frame::frame_type::MANAGEMENT);
        assert_eq!(
            hdr.frame_control.frame_subtype(),
            crate::ieee80211::frame::management_subtype::BEACON
        );

        // Payload starts after header (24 bytes) + Fixed Params (12 bytes) = 36 bytes
        // offset Fixed params: Timestamp (8), Interval (2), Caps (2)
        // Let's verify fixed params roughly
        let interval = u16::from_le_bytes([data[32], data[33]]);
        // 0.102400 seconds = 102.4 ms. Interval is in TUs (1024 microseconds).
        // 1024 us * 100 = 102400 us = 0.1024 s.
        // So expected value is 100 (0x64).
        assert_eq!(interval, 100);

        let caps = u16::from_le_bytes([data[34], data[35]]);
        assert_eq!(caps, 0x0001); // ESS

        // IEs start at offset 36
        let ies_data = &data[36..];
        let mut ie_iter = crate::ieee80211::ie::IeIterator::new(ies_data);

        // Expect SSID "netsim"
        let ssid_ie = ie_iter.next().expect("SSID IE");
        assert_eq!(ssid_ie.id, crate::ieee80211::ie::tags::SSID);
        assert_eq!(ssid_ie.length, 6);
        assert_eq!(ssid_ie.body, b"netsim");

        // No more IEs in this specific packet
        assert!(ie_iter.next().is_none());
    }
}
