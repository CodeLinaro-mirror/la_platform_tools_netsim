// Copyright 2025 The Android Open Source Project

// BLE Constants
const MAX_AD_PAYLOAD_LEN: usize = 31;

// AD Structure Types
const AD_TYPE_FLAGS: u8 = 0x01;
const AD_TYPE_NAME_COMPLETE: u8 = 0x09;
const AD_TYPE_MANUFACTURER_SPECIFIC: u8 = 0xFF;

// Fixed Data
const FLAGS_DATA: [u8; 3] = [0x02, AD_TYPE_FLAGS, 0x06];

/// Helper to construct advertising data payload
pub fn construct_data(manufacturer_data: &[u8], device_name: &Option<String>) -> Vec<u8> {
    let mut data = Vec::new();
    // Flags
    data.extend(FLAGS_DATA);

    // Manufacturer Data
    if !manufacturer_data.is_empty() {
        data.push((manufacturer_data.len() + 1) as u8);
        data.push(AD_TYPE_MANUFACTURER_SPECIFIC);
        data.extend_from_slice(manufacturer_data);
    }

    // Local Name
    if let Some(name) = device_name {
        let name_bytes = name.as_bytes();
        if !name_bytes.is_empty() {
            let mut len = name_bytes.len();
            // Max length check (remaining space)
            // Current data len + 2 (length + type)
            if data.len() + len + 2 > MAX_AD_PAYLOAD_LEN {
                len = MAX_AD_PAYLOAD_LEN - data.len() - 2;
            }
            if len > 0 {
                data.push((len + 1) as u8);
                data.push(AD_TYPE_NAME_COMPLETE); // Complete Local Name
                data.extend_from_slice(&name_bytes[..len]);
            }
        }
    }
    data
}

/// Generates a legacy Bluetooth address from a Chip ID.
///
/// This matches the legacy C++ behavior and netsim daemon behavior where
/// the address is derived from the ID.
/// Example: ID 1000 -> 00:00:00:00:03:e8
pub fn generate_legacy_address(chip_id: u32) -> String {
    format!("00:00:00:00:{:02x}:{:02x}", (chip_id >> 8) & 0xFF, chip_id & 0xFF)
}

/// Generates a default name for a beacon from a Chip ID.
pub fn generate_default_name(chip_id: u32) -> String {
    format!("Beacon-{}", chip_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_data_basic() {
        let data = construct_data(&[], &None);
        assert_eq!(data, vec![0x02, 0x01, 0x06]);
    }

    #[test]
    fn test_construct_data_with_name() {
        let data = construct_data(&[], &Some("Test".to_string()));
        // Flags (3) + Name (2 + 4) = 9 bytes
        assert_eq!(data, vec![0x02, 0x01, 0x06, 0x05, 0x09, b'T', b'e', b's', b't']);
    }

    #[test]
    fn test_construct_data_with_manufacturer_data() {
        let mfg_data = vec![0x01, 0x02];
        let data = construct_data(&mfg_data, &None);
        // Flags (3) + Mfg (2 + 2) = 7 bytes
        assert_eq!(data, vec![0x02, 0x01, 0x06, 0x03, 0xFF, 0x01, 0x02]);
    }

    #[test]
    fn test_construct_data_truncate_name() {
        // Flags = 3 bytes
        // Remaining = 31 - 3 = 28 bytes
        // Name header = 2 bytes
        // Max name len = 26 bytes
        let long_name = "A".repeat(30);
        let data = construct_data(&[], &Some(long_name));
        assert_eq!(data.len(), 31);
        assert_eq!(data[3], 0x1b); // Length = 27 (26 + 1 type)
        assert_eq!(data[4], 0x09);
        assert_eq!(&data[5..], "A".repeat(26).as_bytes());
    }
}

// HCI Constants used for testing and packet parsing

/// HCI Packet Type: Event
pub const HCI_EVENT_PACKET: u8 = 0x04;
/// HCI Event Code: LE Meta Event
pub const LE_META_EVENT: u8 = 0x3E;
/// LE Subevent Code: Advertising Report
pub const LE_ADVERTISING_REPORT: u8 = 0x02;

// Offsets within LE Advertising Report Event
// Byte 0: Packet Type
// Byte 1: Event Code
// Byte 2: Parameter Total Length
// Byte 3: Subevent Code
// Byte 4: Num Reports
// Byte 5: Event Type
// Byte 6: Address Type
// Byte 7-12: Address
/// Offset of the Number of Reports field in an LE Advertising Report
pub const REPORT_NUM_REPORTS_OFFSET: usize = 4;
/// Offset of the Address field in an LE Advertising Report
pub const REPORT_ADDR_OFFSET: usize = 12;

// HCI Commands

/// Set Standard Event Mask (0x0C01) - Enable all events Mask: FF FF FF FF FF FF
/// FF FF
pub const CMD_SET_EVENT_MASK_STD: &[u8] =
    &[0x01, 0x01, 0x0C, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

/// Set LE Event Mask (0x2001) - Enable all LE events Mask: FF 00 00 00 00 00 00
/// 00
pub const CMD_LE_SET_EVENT_MASK: &[u8] =
    &[0x01, 0x01, 0x20, 0x08, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// LE Set Scan Parameters (0x200B)
/// 01 (Cmd) 0B 20 (OpCode) 07 (Len) 00 (Type: Passive) 10 00 (Interval) 10 00
/// (Window) 00 (OwnAddr) 00 (Filter)
pub const CMD_LE_SET_SCAN_PARAMS: &[u8] =
    &[0x01, 0x0B, 0x20, 0x07, 0x00, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00];

/// LE Set Scan Enable (0x200C)
/// 01 (Cmd) 0C 20 (OpCode) 02 (Len) 01 (Enable) 00 (FilterDup)
pub const CMD_LE_SET_SCAN_ENABLE: &[u8] = &[0x01, 0x0C, 0x20, 0x02, 0x01, 0x00];

/// Helper to check if a packet is an LE Advertising Report
pub fn is_le_advertising_report(packet: &[u8]) -> bool {
    packet.len() > REPORT_NUM_REPORTS_OFFSET
        && packet[0] == HCI_EVENT_PACKET
        && packet[1] == LE_META_EVENT
        && packet[3] == LE_ADVERTISING_REPORT
}
