// Copyright 2026 The Android Open Source Project

use common::util::time_display::TimeDisplay;
use netsim_proto::ble_service::{ScanResponse, SniffResponse};
use serde::Serialize;

pub fn print_scan_response(res: &ScanResponse, _verbose: bool) {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let timestamp = TimeDisplay::new(now.as_secs() as i64, now.subsec_nanos()).utc_display_hms();

    let hex_packet = res.packet.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");

    println!("[{}] HCI Packet: {}", timestamp, hex_packet);
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
