// Copyright 2026 The Android Open Source Project

use common::util::time_display::TimeDisplay;
use netsim_proto::ble_service::ScanResponse;

pub fn print_scan_response(res: &ScanResponse, _verbose: bool) {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let timestamp = TimeDisplay::new(now.as_secs() as i64, now.subsec_nanos()).utc_display_hms();

    let hex_packet = res.packet.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");

    println!("[{}] HCI Packet: {}", timestamp, hex_packet);
}
