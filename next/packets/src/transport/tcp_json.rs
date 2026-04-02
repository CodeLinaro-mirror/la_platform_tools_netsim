// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! JSON serialization for TCP headers.

use serde_json::{Map, Value};

use crate::transport::tcp::TcpHeader;

/// Converts a `TcpHeader` to a `serde_json::Map` mimicking `tshark` output.
pub fn to_json(header: &TcpHeader) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("tcp.srcport".to_string(), Value::String(header.source_port.get().to_string()));
    map.insert("tcp.dstport".to_string(), Value::String(header.dest_port.get().to_string()));
    map.insert("tcp.seq".to_string(), Value::String(header.sequence_num.get().to_string()));
    map.insert("tcp.ack".to_string(), Value::String(header.ack_num.get().to_string()));
    map.insert("tcp.hdr_len".to_string(), Value::String(header.header_length().to_string()));
    map.insert("tcp.flags".to_string(), Value::String(format!("0x{:04x}", header.flags())));
    map.insert(
        "tcp.window_size_value".to_string(),
        Value::String(header.window_size.get().to_string()),
    );
    map.insert(
        "tcp.checksum".to_string(),
        Value::String(format!("0x{:04x}", header.checksum.get())),
    );
    map.insert(
        "tcp.urgent_pointer".to_string(),
        Value::String(header.urgent_ptr.get().to_string()),
    );
    map
}
