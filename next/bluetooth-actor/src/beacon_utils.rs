// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// BLE Constants
const MAX_AD_PAYLOAD_LEN: usize = 31;

// AD Structure Types
const AD_TYPE_FLAGS: u8 = 0x01;
const AD_TYPE_NAME_COMPLETE: u8 = 0x09;
const AD_TYPE_TX_POWER_LEVEL: u8 = 0x0A;

const AD_TYPE_MANUFACTURER_SPECIFIC: u8 = 0xFF;

// Fixed Data
const FLAGS_DATA: [u8; 3] = [0x02, AD_TYPE_FLAGS, 0x06];

/// Helper to construct advertising data payload
pub fn construct_data(
    adv_data: &netsim_model::AdvertiseData,
    device_name: &Option<String>,
) -> Vec<u8> {
    let mut data = Vec::new();
    // Flags
    data.extend(FLAGS_DATA);

    // Manufacturer Data
    if !adv_data.manufacturer_data.is_empty() {
        data.push((adv_data.manufacturer_data.len() + 1) as u8);
        data.push(AD_TYPE_MANUFACTURER_SPECIFIC);
        data.extend_from_slice(&adv_data.manufacturer_data);
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
    // Tx Power Level
    if adv_data.include_tx_power_level {
        let max_tx_power = 20; // 20 dBm (Standard default mock power)
        if data.len() + 3 <= MAX_AD_PAYLOAD_LEN {
            data.push(2);
            data.push(AD_TYPE_TX_POWER_LEVEL);
            data.push(max_tx_power as u8);
        }
    }

    // Service UUIDs (Grouped by length into single AD structures)
    let mut uuids_16 = Vec::new();
    let mut uuids_32 = Vec::new();
    let mut uuids_128 = Vec::new();

    for service in &adv_data.services {
        let hex_str = service.uuid.replace("-", "");
        if let Ok(mut uuid_bytes) = hex::decode(&hex_str) {
            uuid_bytes.reverse(); // UUIDs are little-endian in AD
            match uuid_bytes.len() {
                2 => uuids_16.extend(uuid_bytes),
                4 => uuids_32.extend(uuid_bytes),
                16 => uuids_128.extend(uuid_bytes),
                _ => {
                    tracing::error!(
                        "Invalid UUID length {} bytes for advertising data, skipping",
                        uuid_bytes.len()
                    );
                }
            }
        }
    }

    for (class_id, item_size, uuids) in
        [(0x03, 2, uuids_16), (0x05, 4, uuids_32), (0x07, 16, uuids_128)]
    {
        if !uuids.is_empty() {
            let mut add_len = uuids.len();
            if data.len() + add_len + 2 > MAX_AD_PAYLOAD_LEN {
                let available = MAX_AD_PAYLOAD_LEN.saturating_sub(data.len() + 2);
                add_len = (available / item_size) * item_size; // Fit as many
                                                               // whole UUIDs as
                                                               // possible
            }
            if add_len > 0 {
                data.push((add_len + 1) as u8);
                data.push(class_id);
                data.extend(&uuids[..add_len]);
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
        let adv_data = netsim_model::AdvertiseData::default();
        let data = construct_data(&adv_data, &None);
        assert_eq!(data, vec![0x02, 0x01, 0x06]);
    }

    #[test]
    fn test_construct_data_with_name() {
        let mut adv_data = netsim_model::AdvertiseData::default();
        adv_data.include_device_name = true;
        let data = construct_data(&adv_data, &Some("Test".to_string()));
        // Flags (3) + Name (2 + 4) = 9 bytes
        assert_eq!(data, vec![0x02, 0x01, 0x06, 0x05, 0x09, b'T', b'e', b's', b't']);
    }

    #[test]
    fn test_construct_data_with_manufacturer_data() {
        let mut adv_data = netsim_model::AdvertiseData::default();
        adv_data.manufacturer_data = vec![0x01, 0x02];
        let data = construct_data(&adv_data, &None);
        // Flags (3) + Mfg (2 + 2) = 7 bytes
        assert_eq!(data, vec![0x02, 0x01, 0x06, 0x03, 0xFF, 0x01, 0x02]);
    }

    #[test]
    fn test_construct_data_truncate_name() {
        // Flags = 3 bytes
        // Remaining = 31 - 3 = 28 bytes
        // Name header = 2 bytes
        // Max name len = 26 bytes
        let adv_data = netsim_model::AdvertiseData::default();
        let long_name = "A".repeat(30);
        let data = construct_data(&adv_data, &Some(long_name));
        assert_eq!(data.len(), 31);
        assert_eq!(data[3], 0x1b); // Length = 27 (26 + 1 type)
        assert_eq!(data[4], 0x09);
        assert_eq!(&data[5..], "A".repeat(26).as_bytes());
    }

    #[test]
    fn test_construct_data_with_tx_power_level() {
        let mut adv_data = netsim_model::AdvertiseData::default();
        adv_data.include_tx_power_level = true;
        let data = construct_data(&adv_data, &None);
        // Flags (3) + Tx Power (3) = 6 bytes
        assert_eq!(data, vec![0x02, 0x01, 0x06, 0x02, 0x0A, 20]);
    }

    #[test]
    fn test_construct_data_with_service_uuids() {
        let mut adv_data = netsim_model::AdvertiseData::default();
        adv_data.services.push(netsim_model::Service {
            uuid: "180D".to_string(), // Heart Rate Service 16-bit UUID
            data: vec![0xAB, 0xCD],
        });
        adv_data.services.push(netsim_model::Service {
            uuid: "180F".to_string(), // Battery Service 16-bit UUID
            data: vec![],
        });
        let data = construct_data(&adv_data, &None);
        // Flags (3) + Service (1 len + 1 type + 4 uuid elements) = 9 bytes
        // 180D -> 0x0D, 0x18
        // 180F -> 0x0F, 0x18
        assert_eq!(data, vec![0x02, 0x01, 0x06, 0x05, 0x03, 0x0D, 0x18, 0x0F, 0x18]);
    }

    #[test]
    fn test_construct_data_with_invalid_service_uuid() {
        let mut adv_data = netsim_model::AdvertiseData::default();
        adv_data.services.push(netsim_model::Service {
            uuid: "180".to_string(), // Invalid odd-length string
            data: vec![],
        });
        adv_data.services.push(netsim_model::Service {
            uuid: "180D11".to_string(), // 3 bytes (invalid specification length)
            data: vec![],
        });
        let data = construct_data(&adv_data, &None);
        // Invalid services should be skipped. Only Flags remain.
        assert_eq!(data, vec![0x02, 0x01, 0x06]);
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
