// Copyright 2025 The Android Open Source Project

//! JSON serialization for UDP headers.

use serde_json::{Map, Value};

use crate::transport::udp::UdpHeader;

/// Converts a `UdpHeader` to a `serde_json::Map` mimicking `tshark` output.
pub fn to_json(header: &UdpHeader) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("udp.srcport".to_string(), Value::String(header.source_port.get().to_string()));
    map.insert("udp.dstport".to_string(), Value::String(header.dest_port.get().to_string()));
    map.insert("udp.length".to_string(), Value::String(header.length.get().to_string()));
    map.insert(
        "udp.checksum".to_string(),
        Value::String(format!("0x{:04x}", header.checksum.get())),
    );
    map
}
