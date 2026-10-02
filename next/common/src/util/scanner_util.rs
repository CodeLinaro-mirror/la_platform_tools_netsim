// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

const HCI_EVENT_PACKET: u8 = 0x04;
const HCI_LE_META_EVENT: u8 = 0x3E;
const HCI_LE_ADVERTISING_REPORT: u8 = 0x02;

const HCI_HEADER_MIN_LEN: usize = 5;
const REPORT_HEADER_LEN: usize = 9;
const MAC_ADDR_OFFSET: usize = 2;
const MAC_ADDR_LEN: usize = 6;
const PAYLOAD_LEN_OFFSET: usize = 8;
const RSSI_LEN: usize = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct ScanResult {
    pub mac: [u8; 6],
    pub event_type: u8,
    pub rssi: i8,
    pub payload: Vec<u8>,
}

pub fn parse_hci_scan_report(data: &[u8]) -> Result<Vec<ScanResult>, &'static str> {
    // 1. Validation: Min length check
    // Header (3) + Subevent (1) + NumReports (1) = 5 bytes minimum structure
    if data.len() < HCI_HEADER_MIN_LEN {
        return Err("Packet too short");
    }

    // 2. Validate HCI Header
    if data[0] != HCI_EVENT_PACKET {
        return Err("Not an HCI Event packet");
    }
    if data[1] != HCI_LE_META_EVENT {
        return Err("Not an LE Meta Event");
    }
    if data[3] != HCI_LE_ADVERTISING_REPORT {
        return Err("Not an LE Advertising Report");
    }

    // 3. Extract Num Reports
    let num_reports = data[4] as usize;
    let mut results = Vec::new();
    let mut cursor = HCI_HEADER_MIN_LEN;

    for _ in 0..num_reports {
        // Check if cursor has enough bytes for fixed report header
        // (Event Type [1] + Addr Type [1] + Addr [6] + Data Len [1] = 9 bytes)
        if cursor + REPORT_HEADER_LEN > data.len() {
            return Err("Packet truncated: header exceeds buffer");
        }

        let event_type = data[cursor];
        let mut mac = [0u8; 6];
        mac.copy_from_slice(
            &data[cursor + MAC_ADDR_OFFSET..cursor + MAC_ADDR_OFFSET + MAC_ADDR_LEN],
        );
        // Packet is already Big Endian (human readable), do not reverse.

        let payload_len = data[cursor + PAYLOAD_LEN_OFFSET] as usize;
        let payload_start = cursor + REPORT_HEADER_LEN;
        let payload_end = payload_start + payload_len;

        // Check payload + RSSI byte (1 byte)
        if payload_end + RSSI_LEN > data.len() {
            return Err("Packet truncated: payload exceeds buffer");
        }

        let payload = data[payload_start..payload_end].to_vec();
        let rssi = data[payload_end] as i8;

        results.push(ScanResult { mac, event_type, rssi, payload });

        // Advance cursor: Header (9) + Payload (len) + RSSI (1)
        cursor += REPORT_HEADER_LEN + payload_len + RSSI_LEN;
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADV_IND: u8 = 0x00;
    const ADV_DIRECT_IND: u8 = 0x01;
    const ADV_SCAN_IND: u8 = 0x02;

    const ADDR_TYPE_PUBLIC: u8 = 0x00;
    const ADDR_TYPE_RANDOM: u8 = 0x01;

    const TEST_MAC_1: [u8; 6] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
    const TEST_MAC_2: [u8; 6] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    const TEST_MAC_3: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];

    const SUBEVENT_CODE_LEN: usize = 1;
    const NUM_REPORTS_LEN: usize = 1;

    struct TestReport<'a> {
        event_type: u8,
        address_type: u8,
        mac: [u8; 6],
        payload: &'a [u8],
        rssi: i8,
    }

    impl<'a> TestReport<'a> {
        fn to_bytes(&self) -> impl Iterator<Item = u8> + 'a {
            [self.event_type, self.address_type]
                .into_iter()
                .chain(self.mac)
                .chain([self.payload.len() as u8])
                .chain(self.payload.iter().copied())
                .chain([self.rssi as u8])
        }
    }

    fn build_hci_scan_report_packet(
        num_reports: u8,
        reports: impl IntoIterator<Item = u8>,
    ) -> Vec<u8> {
        let reports: Vec<u8> = reports.into_iter().collect();
        let param_len = (SUBEVENT_CODE_LEN + NUM_REPORTS_LEN + reports.len()) as u8;
        [HCI_EVENT_PACKET, HCI_LE_META_EVENT, param_len, HCI_LE_ADVERTISING_REPORT, num_reports]
            .into_iter()
            .chain(reports)
            .collect()
    }

    #[test]
    fn test_parse_hci_scan_report_single_success() {
        let report = TestReport {
            event_type: ADV_IND,
            address_type: ADDR_TYPE_PUBLIC,
            mac: TEST_MAC_1,
            payload: &[0x02, 0x01, 0x06],
            rssi: -50,
        };
        let packet = build_hci_scan_report_packet(1, report.to_bytes());

        let results = parse_hci_scan_report(&packet).expect("Failed to parse valid scan report");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            ScanResult {
                mac: TEST_MAC_1,
                event_type: ADV_IND,
                rssi: -50,
                payload: vec![0x02, 0x01, 0x06],
            }
        );
    }

    #[test]
    fn test_parse_hci_scan_report_multi_success() {
        let report1 = TestReport {
            event_type: ADV_DIRECT_IND,
            address_type: ADDR_TYPE_RANDOM,
            mac: TEST_MAC_2,
            payload: &[0x10, 0x20],
            rssi: -60,
        };
        let report2 = TestReport {
            event_type: ADV_SCAN_IND,
            address_type: ADDR_TYPE_PUBLIC,
            mac: TEST_MAC_3,
            payload: &[0xFF],
            rssi: -70,
        };
        let packet = build_hci_scan_report_packet(2, report1.to_bytes().chain(report2.to_bytes()));

        let results = parse_hci_scan_report(&packet).expect("Failed to parse valid multi reports");
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0],
            ScanResult {
                mac: TEST_MAC_2,
                event_type: ADV_DIRECT_IND,
                rssi: -60,
                payload: vec![0x10, 0x20],
            }
        );
        assert_eq!(
            results[1],
            ScanResult {
                mac: TEST_MAC_3,
                event_type: ADV_SCAN_IND,
                rssi: -70,
                payload: vec![0xFF],
            }
        );
    }

    #[test]
    fn test_parse_hci_scan_report_empty_payload() {
        let report = TestReport {
            event_type: ADV_DIRECT_IND,
            address_type: ADDR_TYPE_PUBLIC,
            mac: TEST_MAC_1,
            payload: &[],
            rssi: -45,
        };
        let packet = build_hci_scan_report_packet(1, report.to_bytes());

        let results = parse_hci_scan_report(&packet).expect("Failed to parse empty payload report");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            ScanResult { mac: TEST_MAC_1, event_type: ADV_DIRECT_IND, rssi: -45, payload: vec![] }
        );
    }

    #[test]
    fn test_parse_hci_scan_report_zero_reports() {
        let packet = build_hci_scan_report_packet(0, std::iter::empty());
        let results = parse_hci_scan_report(&packet).expect("Failed to parse zero reports");
        assert_eq!(results, vec![]);
    }

    #[test]
    fn test_parse_hci_scan_report_packet_too_short() {
        assert_eq!(parse_hci_scan_report(&[]), Err("Packet too short"));
        assert_eq!(parse_hci_scan_report(&[HCI_EVENT_PACKET]), Err("Packet too short"));
        assert_eq!(
            parse_hci_scan_report(&[HCI_EVENT_PACKET, HCI_LE_META_EVENT]),
            Err("Packet too short")
        );
        assert_eq!(
            parse_hci_scan_report(&[HCI_EVENT_PACKET, HCI_LE_META_EVENT, 0x00]),
            Err("Packet too short")
        );
        assert_eq!(
            parse_hci_scan_report(&[
                HCI_EVENT_PACKET,
                HCI_LE_META_EVENT,
                0x00,
                HCI_LE_ADVERTISING_REPORT
            ]),
            Err("Packet too short")
        );
    }

    #[test]
    fn test_parse_hci_scan_report_invalid_packet_type() {
        const INVALID_PACKET_TYPE: u8 = 0x01; // HCI Command instead of Event
        let packet =
            [INVALID_PACKET_TYPE, HCI_LE_META_EVENT, 0x02, HCI_LE_ADVERTISING_REPORT, 0x00];
        assert_eq!(parse_hci_scan_report(&packet), Err("Not an HCI Event packet"));
    }

    #[test]
    fn test_parse_hci_scan_report_invalid_meta_event() {
        const INVALID_EVENT_CODE: u8 = 0x3F; // Not LE Meta Event
        let packet = [HCI_EVENT_PACKET, INVALID_EVENT_CODE, 0x02, HCI_LE_ADVERTISING_REPORT, 0x00];
        assert_eq!(parse_hci_scan_report(&packet), Err("Not an LE Meta Event"));
    }

    #[test]
    fn test_parse_hci_scan_report_invalid_subevent() {
        const INVALID_SUBEVENT_CODE: u8 = 0x03; // Not LE Advertising Report
        let packet = [HCI_EVENT_PACKET, HCI_LE_META_EVENT, 0x02, INVALID_SUBEVENT_CODE, 0x00];
        assert_eq!(parse_hci_scan_report(&packet), Err("Not an LE Advertising Report"));
    }

    #[test]
    fn test_parse_hci_scan_report_truncated_header() {
        // Only provide 7 of the 9 required report header bytes after num_reports.
        let incomplete_report_header: Vec<u8> = [ADV_IND, ADDR_TYPE_PUBLIC]
            .into_iter()
            .chain(TEST_MAC_1[..5].iter().copied())
            .collect();
        let param_len =
            (SUBEVENT_CODE_LEN + NUM_REPORTS_LEN + incomplete_report_header.len()) as u8;
        let packet: Vec<u8> =
            [HCI_EVENT_PACKET, HCI_LE_META_EVENT, param_len, HCI_LE_ADVERTISING_REPORT, 1]
                .into_iter()
                .chain(incomplete_report_header)
                .collect();
        assert_eq!(parse_hci_scan_report(&packet), Err("Packet truncated: header exceeds buffer"));
    }

    #[test]
    fn test_parse_hci_scan_report_truncated_payload() {
        // Declare payload_len = 5, but only provide 3 payload bytes (missing 2 payload bytes and 1 RSSI byte).
        let declared_payload_len: u8 = 5;
        let actual_payload = [0x01, 0x02, 0x03];
        let report_prefix = [ADV_IND, ADDR_TYPE_PUBLIC]
            .into_iter()
            .chain(TEST_MAC_1)
            .chain([declared_payload_len])
            .chain(actual_payload);
        let packet = build_hci_scan_report_packet(1, report_prefix);
        assert_eq!(parse_hci_scan_report(&packet), Err("Packet truncated: payload exceeds buffer"));
    }

    #[test]
    fn test_parse_hci_scan_report_truncated_rssi() {
        // Provide exact payload bytes matching payload_len, but omit the trailing RSSI byte.
        let payload = [0x01, 0x02];
        let report_without_rssi = [ADV_IND, ADDR_TYPE_PUBLIC]
            .into_iter()
            .chain(TEST_MAC_1)
            .chain([payload.len() as u8])
            .chain(payload);
        let packet = build_hci_scan_report_packet(1, report_without_rssi);
        assert_eq!(parse_hci_scan_report(&packet), Err("Packet truncated: payload exceeds buffer"));
    }

    #[test]
    fn test_parse_hci_scan_report_multi_truncated_second_report_header() {
        let valid_report = TestReport {
            event_type: ADV_IND,
            address_type: ADDR_TYPE_PUBLIC,
            mac: TEST_MAC_1,
            payload: &[0x01, 0x02],
            rssi: -50,
        };
        // Report 2 provides only 4 of the 9 required header bytes
        let incomplete_second_report: Vec<u8> = [ADV_SCAN_IND, ADDR_TYPE_PUBLIC]
            .into_iter()
            .chain(TEST_MAC_2[..2].iter().copied())
            .collect();
        let param_len = (SUBEVENT_CODE_LEN
            + NUM_REPORTS_LEN
            + (REPORT_HEADER_LEN + valid_report.payload.len() + RSSI_LEN)
            + incomplete_second_report.len()) as u8;
        let packet: Vec<u8> =
            [HCI_EVENT_PACKET, HCI_LE_META_EVENT, param_len, HCI_LE_ADVERTISING_REPORT, 2]
                .into_iter()
                .chain(valid_report.to_bytes())
                .chain(incomplete_second_report)
                .collect();
        assert_eq!(parse_hci_scan_report(&packet), Err("Packet truncated: header exceeds buffer"));
    }

    #[test]
    fn test_parse_hci_scan_report_multi_truncated_second_report_payload() {
        let valid_report = TestReport {
            event_type: ADV_IND,
            address_type: ADDR_TYPE_PUBLIC,
            mac: TEST_MAC_1,
            payload: &[0x01, 0x02],
            rssi: -50,
        };
        let declared_payload_len: u8 = 4;
        let actual_payload = [0xAA, 0xBB]; // 2 bytes instead of 4 + 1 RSSI
        let incomplete_second_report = [ADV_SCAN_IND, ADDR_TYPE_RANDOM]
            .into_iter()
            .chain(TEST_MAC_3)
            .chain([declared_payload_len])
            .chain(actual_payload);
        let param_len = (SUBEVENT_CODE_LEN
            + NUM_REPORTS_LEN
            + (REPORT_HEADER_LEN + valid_report.payload.len() + RSSI_LEN)
            + (REPORT_HEADER_LEN + actual_payload.len())) as u8;
        let packet: Vec<u8> =
            [HCI_EVENT_PACKET, HCI_LE_META_EVENT, param_len, HCI_LE_ADVERTISING_REPORT, 2]
                .into_iter()
                .chain(valid_report.to_bytes())
                .chain(incomplete_second_report)
                .collect();
        assert_eq!(parse_hci_scan_report(&packet), Err("Packet truncated: payload exceeds buffer"));
    }
}
