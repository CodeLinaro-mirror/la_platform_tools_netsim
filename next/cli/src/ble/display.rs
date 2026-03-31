// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use common::util::time_display::TimeDisplay;
use netsim_packets::hci::{
    events::{parse_hci_event, HciEvent, LeMetaEvent},
    types::{GapDataType, LeAdvertisingEventType, OwnAddressType},
};
use netsim_proto::ble_service::{ScanResponse, SniffResponse};
use serde::Serialize;

const H4_EVENT_PACKET_TYPE: u8 = 0x04;
const SCAN_RSP_KEY_OFFSET: u16 = 0x0100;

pub struct BleEvent {
    pub timestamp_str: String,
    pub addr_display: String,
    pub event_type_str: String,
    pub rssi: i8,
    pub data_len: usize,
    pub raw_hex: String,
    pub ad_payloads: BTreeMap<u16, AdPayload>,
}

#[derive(Clone)]
pub struct AdPayload {
    pub ad_type: u8,
    pub type_str: String,
    pub decoded_val: String,
}

pub fn parse_scan_response(res: &ScanResponse, verbose: bool) -> Vec<BleEvent> {
    let mut events = Vec::new();
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let timestamp = TimeDisplay::new(now.as_secs() as i64, now.subsec_nanos()).utc_display_hms();

    let packet = &res.packet;
    let hex_packet = if verbose {
        packet.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    };

    let mut offset = 0;
    if !packet.is_empty() && packet[0] == H4_EVENT_PACKET_TYPE {
        offset += 1;
    }

    let reports = match parse_hci_event(&packet[offset..]) {
        Some(HciEvent::LeMetaEvent(LeMetaEvent::LeAdvertisingReport(reports))) => reports,
        _ => return events,
    };

    for report in reports {
        let addr_display = format!(
            "{} ({})",
            report.address,
            if report.address_type == OwnAddressType::PUBLIC_DEVICE_ADDRESS {
                "Public"
            } else {
                "Random"
            }
        );

        let event_type_str = report.event_type.to_string();
        let is_scan_rsp = report.event_type == LeAdvertisingEventType::SCAN_RSP;

        let ad_payloads = if !report.data.is_empty() {
            parse_ad_structures(report.data, is_scan_rsp)
        } else {
            BTreeMap::new()
        };

        events.push(BleEvent {
            timestamp_str: timestamp.clone(),
            addr_display,
            event_type_str,
            rssi: report.rssi,
            data_len: report.data_length as usize,
            raw_hex: hex_packet.clone(),
            ad_payloads,
        });
    }

    events
}

fn parse_ad_structures(data: &[u8], is_scan_rsp: bool) -> BTreeMap<u16, AdPayload> {
    let mut i = 0;
    let mut outputs = BTreeMap::new();
    while i < data.len() {
        let len = data[i] as usize;
        if len == 0 || i + 1 + len > data.len() {
            break;
        }
        let ad_type = data[i + 1];
        let ad_data = &data[i + 2..i + 1 + len];

        let type_str = GapDataType(ad_type).to_string();

        let decoded_val = match GapDataType(ad_type) {
            GapDataType::SHORTENED_LOCAL_NAME | GapDataType::COMPLETE_LOCAL_NAME => {
                String::from_utf8_lossy(ad_data).to_string()
            }
            GapDataType::TX_POWER_LEVEL => {
                if ad_data.len() == 1 {
                    format!("{} dBm", ad_data[0] as i8)
                } else {
                    format_hex_data(ad_data)
                }
            }
            GapDataType::INCOMPLETE_16BIT_UUIDS | GapDataType::COMPLETE_16BIT_UUIDS
                if ad_data.len() % 2 == 0 =>
            {
                let mut uuids = Vec::new();
                for chunk in ad_data.chunks(2) {
                    uuids.push(format!("0x{:02x}{:02x}", chunk[1], chunk[0]));
                }
                uuids.join(", ")
            }
            GapDataType::INCOMPLETE_32BIT_UUIDS | GapDataType::COMPLETE_32BIT_UUIDS
                if ad_data.len() % 4 == 0 =>
            {
                let mut uuids = Vec::new();
                for chunk in ad_data.chunks(4) {
                    uuids.push(format!(
                        "0x{:02x}{:02x}{:02x}{:02x}",
                        chunk[3], chunk[2], chunk[1], chunk[0]
                    ));
                }
                uuids.join(", ")
            }
            GapDataType::INCOMPLETE_128BIT_UUIDS | GapDataType::COMPLETE_128BIT_UUIDS
                if ad_data.len() % 16 == 0 =>
            {
                let mut uuids = Vec::new();
                for chunk in ad_data.chunks(16) {
                    uuids.push(format_128bit_uuid(chunk));
                }
                uuids.join(", ")
            }
            GapDataType::SERVICE_DATA_16BIT if ad_data.len() >= 2 => {
                let uuid = format!("0x{:02x}{:02x}", ad_data[1], ad_data[0]);
                format_uuid_service_data(uuid, &ad_data[2..])
            }
            GapDataType::SERVICE_DATA_32BIT if ad_data.len() >= 4 => {
                let uuid = format!(
                    "0x{:02x}{:02x}{:02x}{:02x}",
                    ad_data[3], ad_data[2], ad_data[1], ad_data[0]
                );
                format_uuid_service_data(uuid, &ad_data[4..])
            }
            GapDataType::SERVICE_DATA_128BIT if ad_data.len() >= 16 => {
                let uuid = format_128bit_uuid(&ad_data[..16]);
                format_uuid_service_data(uuid, &ad_data[16..])
            }
            _ => format_hex_data(ad_data),
        };

        let map_key =
            if is_scan_rsp { SCAN_RSP_KEY_OFFSET | (ad_type as u16) } else { ad_type as u16 };

        outputs.insert(map_key, AdPayload { ad_type, type_str, decoded_val });

        i += 1 + len;
    }
    outputs
}

fn format_128bit_uuid(uuid: &[u8]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid[15], uuid[14], uuid[13], uuid[12], uuid[11], uuid[10], uuid[9], uuid[8],
        uuid[7], uuid[6], uuid[5], uuid[4], uuid[3], uuid[2], uuid[1], uuid[0]
    )
}

fn format_uuid_service_data(uuid: String, data: &[u8]) -> String {
    let data_str = if data.is_empty() { String::from("None") } else { format_hex_data(data) };
    format!("UUID: {}, Data: {}", uuid, data_str)
}

fn format_hex_data(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
}

pub fn print_json_event(event: &BleEvent, verbose: bool) {
    use serde::Serialize;

    #[derive(Serialize)]
    struct JsonAdPayload<'a> {
        type_id: u8,
        type_name: &'a str,
        value: &'a str,
    }

    #[derive(Serialize)]
    struct JsonBleEvent<'a> {
        timestamp: &'a str,
        address: &'a str,
        event_type: &'a str,
        rssi: i8,
        data_len: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        raw_hex: Option<&'a str>,
        ad_payloads: Vec<JsonAdPayload<'a>>,
    }

    let out = JsonBleEvent {
        timestamp: &event.timestamp_str,
        address: &event.addr_display,
        event_type: &event.event_type_str,
        rssi: event.rssi,
        data_len: event.data_len,
        raw_hex: if verbose { Some(&event.raw_hex) } else { None },
        ad_payloads: event
            .ad_payloads
            .values()
            .map(|ad| JsonAdPayload {
                type_id: ad.ad_type,
                type_name: &ad.type_str,
                value: &ad.decoded_val,
            })
            .collect(),
    };

    match serde_json::to_string(&out) {
        Ok(json_str) => println!("{}", json_str),
        Err(e) => eprintln!("Failed to serialize BLE event to JSON: {}", e),
    }
}

pub fn print_sniff_response(res: &SniffResponse, _verbose: bool) {
    let duration = std::time::Duration::from_micros(res.timestamp as u64);
    let timestamp =
        TimeDisplay::new(duration.as_secs() as i64, duration.subsec_nanos()).utc_display_hms();
    let hex_packet = res.packet.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("");

    let packet_type_str = match res.packet.first().copied() {
        Some(0x00) => "UNKNOWN",
        Some(0x01) => "ACL",
        Some(0x02) => "SCO",
        Some(0x03) => "LE_CONNECTED_ISOCHRONOUS_PDU",
        Some(0x04) => "LE_BROADCAST_ISOCHRONOUS_PDU",
        Some(0x05) => "DISCONNECT",
        Some(0x06) => "INQUIRY",
        Some(0x07) => "INQUIRY_RESPONSE",
        Some(0x0B) => "LE_LEGACY_ADVERTISING_PDU",
        Some(0x37) => "LE_EXTENDED_ADVERTISING_PDU",
        Some(0x40) => "LE_PERIODIC_ADVERTISING_PDU",
        Some(0x0C) => "LE_CONNECT",
        Some(0x0D) => "LE_CONNECT_COMPLETE",
        Some(0x0E) => "LE_SCAN",
        Some(0x0F) => "LE_SCAN_RESPONSE",
        Some(0x10) => "PAGE",
        Some(0x11) => "PAGE_RESPONSE",
        Some(0x12) => "PAGE_REJECT",
        Some(0x1D) => "REMOTE_NAME_REQUEST",
        Some(0x1E) => "REMOTE_NAME_REQUEST_RESPONSE",
        Some(0x34) => "LMP",
        Some(0x41) => "LLCP",
        None => "EMPTY",
        Some(_) => "", // Fallback correctly handled below
    };

    let packet_type = if packet_type_str.is_empty() {
        format!("0x{:02X}", res.packet.first().unwrap())
    } else {
        packet_type_str.to_string()
    };

    #[derive(Serialize)]
    struct JsonSniffResponse {
        timestamp: String,
        packet_type: String,
        baseband_packet: String,
        length: usize,
    }

    let out = JsonSniffResponse {
        timestamp: timestamp.to_string(),
        packet_type,
        baseband_packet: hex_packet,
        length: res.packet.len(),
    };

    if let Ok(json_str) = serde_json::to_string(&out) {
        println!("{}", json_str);
    } else {
        println!("[{}] Baseband Packet: {}", timestamp, out.baseband_packet);
    }
}
