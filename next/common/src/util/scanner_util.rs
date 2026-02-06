const HCI_EVENT_PACKET: u8 = 0x04;
const HCI_LE_META_EVENT: u8 = 0x3E;
const HCI_LE_ADVERTISING_REPORT: u8 = 0x02;

pub struct ScanResult {
    pub mac: [u8; 6],
    pub rssi: i8,
    pub payload: Vec<u8>,
}

pub fn parse_hci_scan_report(data: &[u8]) -> Result<Vec<ScanResult>, &'static str> {
    // 1. Validation: Min length check
    // Header (3) + Subevent (1) + NumReports (1) = 5 bytes minimum
    // structure
    if data.len() < 5 {
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
    let mut cursor = 5;

    for _ in 0..num_reports {
        // Check if cursor has enough bytes for fixed headers
        // (Event Type + Addr Type + Addr + Data Len = 1 + 1 + 6 + 1 = 9
        // bytes)
        if cursor + 9 > data.len() {
            return Err("Packet truncated: header exceeds buffer");
        }

        let mut mac = [0u8; 6];
        mac.copy_from_slice(&data[cursor + 2..cursor + 8]);
        // Packet is already Big Endian (human readable), do not reverse.

        let payload_len = data[cursor + 8] as usize;
        let payload_start = cursor + 9;
        let payload_end = payload_start + payload_len;

        // Check payload + RSSI byte (1 byte)
        if payload_end + 1 > data.len() {
            return Err("Packet truncated: payload exceeds buffer");
        }

        let payload = data[payload_start..payload_end].to_vec();
        let rssi = data[payload_end] as i8;

        results.push(ScanResult { mac, rssi, payload });

        // Advance cursor: Header (9) + Payload (len) + RSSI (1)
        cursor += 9 + payload_len + 1;
    }

    Ok(results)
}
