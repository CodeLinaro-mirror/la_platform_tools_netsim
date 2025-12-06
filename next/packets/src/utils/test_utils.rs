// Copyright 2025 The Android Open Source Project

use crate::packet;
use crate::packet::json::to_json;
use std::fs::File;

use std::path::{Path, PathBuf};

/// Validates that the JSON output from parsing a PCAP file matches the golden JSON file.
///
/// # Arguments
///
/// * `pcap_path` - Path to the PCAP file.
/// * `json_path` - Path to the golden JSON file (expected output from tshark).
/// * `layer_name` - The specific layer to validate (e.g., "icmp", "tcp").
pub fn validate_pcap_json(pcap_path: PathBuf, json_path: PathBuf, fields: &[&str]) {
    if !pcap_path.exists() || !json_path.exists() {
        println!("Skipping validation for {:?}: test data not found", pcap_path);
        return;
    }

    // Read golden JSON
    let file = File::open(json_path).expect("Failed to open JSON file");
    let tshark_json: serde_json::Value =
        serde_json::from_reader(file).expect("Failed to parse JSON file");

    // Read PCAP file
    let file = File::open(&pcap_path).expect("Failed to open PCAP file");
    let mut reader = crate::pcap::PcapReader::new(file).expect("Failed to create PcapReader");

    // Read first record
    let (_, packet_data) =
        reader.next_record().expect("Failed to read PCAP record").expect("No records in PCAP");

    // Get LinkType (might be None if PCAPNG and no IDB yet, but usually IDB is first)
    let link_type = reader
        .link_type()
        .expect("Unknown LinkType: PCAPNG file missing Interface Description Block?");

    let netsim_json = match link_type {
        crate::pcap::frame::LINKTYPE_IEEE802_11 => {
            let packet = crate::ieee80211::Ieee80211::decode(&packet_data)
                .expect("Failed to parse 802.11 packet");
            crate::ieee80211::json::to_json(&packet, packet_data.len())
        }
        crate::pcap::frame::LINKTYPE_ETHERNET => {
            let packet = packet::parse(&packet_data).expect("Failed to parse packet");
            to_json(&packet, packet_data.len())
        }
        crate::pcap::frame::LINKTYPE_RADIOTAP => {
            // Parse Radiotap header to find length
            use crate::pcap::radiotap::RadiotapHeader;

            if let Ok((header, _)) =
                zerocopy::Ref::<&[u8], RadiotapHeader>::from_prefix(&packet_data)
            {
                let len = header.len as usize;
                if len <= packet_data.len() {
                    let inner_data = &packet_data[len..];
                    // Assume inner is 802.11
                    let packet = crate::ieee80211::Ieee80211::decode(inner_data)
                        .expect("Failed to parse inner 802.11 packet from Radiotap");
                    crate::ieee80211::json::to_json(&packet, packet_data.len())
                } else {
                    panic!(
                        "Radiotap header length {} exceeds packet length {}",
                        len,
                        packet_data.len()
                    );
                }
            } else {
                panic!("Failed to parse Radiotap header");
            }
        }
        crate::pcap::frame::LINKTYPE_NETLINK => {
            // Netlink parsing
            crate::netlink::nl80211_json::packet_to_json(&packet_data)
        }
        _ => panic!("Unsupported LinkType: {}", link_type),
    };

    let netsim_source = &netsim_json[0]["_source"]["layers"];
    let tshark_source = &tshark_json[0]["_source"]["layers"];

    // Compare specific fields
    for field in fields {
        let n_val = find_value(netsim_source, field);
        let t_val = find_value(tshark_source, field);

        match (n_val, t_val) {
            (Some(n), Some(t)) => {
                if !compare_values(n, t) {
                    panic!(
                        "Field mismatch: {} (netsim: {:?}, tshark: {:?}) in {:?}",
                        field, n, t, pcap_path
                    );
                }
            }
            (None, Some(_)) => panic!("Field missing in netsim: {} in {:?}", field, pcap_path),
            (Some(_), None) => panic!("Field missing in tshark: {} in {:?}", field, pcap_path),
            (None, None) => panic!("Field missing in both: {} in {:?}", field, pcap_path),
        }
    }
}

fn find_value<'a>(layers: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if let Some(obj) = layers.as_object() {
        if let Some(v) = obj.get(key) {
            return Some(v);
        }
        for v in obj.values() {
            if let Some(res) = find_value(v, key) {
                return Some(res);
            }
        }
    }
    None
}

fn compare_json_objects(netsim: &serde_json::Value, tshark: &serde_json::Value, path: &Path) {
    match netsim {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                // Skip "frame" layer metadata that we can't reproduce exactly (time, numbering)
                // But we DO want to validate frame.len, frame.cap_len, frame.protocols
                if key == "frame" {
                    if let Some(t_frame) = tshark.get("frame") {
                        // Validate specific fields
                        let fields_to_check = ["frame.len", "frame.cap_len", "frame.protocols"];
                        for field in fields_to_check {
                            if let Some(n_val) = val.get(field) {
                                if let Some(t_val) = t_frame.get(field) {
                                    if !compare_values(n_val, t_val) {
                                        panic!(
                                            "Frame field mismatch: {} (netsim: {:?}, tshark: {:?})",
                                            field, n_val, t_val
                                        );
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }

                // Special handling for keys that might be nested in tshark output
                // or named slightly differently.
                // We search for the key recursively in tshark object.
                let found = find_value(tshark, key); // Use the new find_value
                if let Some(t_val) = found {
                    if !compare_values(val, t_val) {
                        panic!("Field '{}' mismatch in tshark output. Path: {:?}", key, path);
                    }
                } else {
                    // If not found, it's a failure unless it's a known extra field
                    if key.ends_with("_hex") {
                        // Skip hex fields if not found (internal representation)
                        continue;
                    }
                    panic!("Field '{}' not found in tshark output. Path: {:?}", key, path);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            // For arrays, we expect tshark to have a matching array?
            // Or maybe netsim array corresponds to something else.
            // Currently netsim output is mostly objects.
            // If we have an array, we try to match by index.
            if let serde_json::Value::Array(t_arr) = tshark {
                assert_eq!(arr.len(), t_arr.len(), "Array length mismatch at {:?}", path);
                for (i, val) in arr.iter().enumerate() {
                    compare_json_objects(val, &t_arr[i], path);
                }
            } else {
                // Tshark might represent single-element array as object or vice versa?
                // For now, strict type check.
                panic!("Type mismatch at {:?}: expected Array", path);
            }
        }
        _ => {
            // Primitive values are compared in find_and_compare usually,
            // but if we reached here via array iteration:
            assert_eq!(netsim, tshark, "Value mismatch at {:?}", path);
        }
    }
}

fn compare_values(n_val: &serde_json::Value, t_val: &serde_json::Value) -> bool {
    match (n_val, t_val) {
        (serde_json::Value::String(n_str), serde_json::Value::String(t_str)) => {
            n_str.eq_ignore_ascii_case(t_str)
        }
        (serde_json::Value::Number(n_num), serde_json::Value::String(t_str)) => {
            // Tshark often stores numbers as strings (e.g. "0")
            // Try to parse t_str as number or convert n_num to string
            if let Ok(t_num) = t_str.parse::<f64>() {
                if let Some(n_f64) = n_num.as_f64() {
                    return (n_f64 - t_num).abs() < f64::EPSILON;
                }
            }
            // Handle hex strings (e.g. "0x0003")
            if let Some(stripped) = t_str.strip_prefix("0x") {
                if let Ok(t_int) = u64::from_str_radix(stripped, 16) {
                    if let Some(n_u64) = n_num.as_u64() {
                        return n_u64 == t_int;
                    }
                }
            }
            // Fallback: compare as strings
            n_num.to_string() == *t_str
        }
        (serde_json::Value::Bool(n_bool), serde_json::Value::String(t_str)) => {
            // Tshark might use "1"/"0" for bools?
            if *n_bool {
                t_str == "1" || t_str.eq_ignore_ascii_case("true")
            } else {
                t_str == "0" || t_str.eq_ignore_ascii_case("false")
            }
        }
        (serde_json::Value::Object(_), serde_json::Value::Object(_)) => {
            // If both are objects, we recurse
            compare_json_objects(n_val, t_val, Path::new(""));
            true
        }
        (n, t) => n == t,
    }
}
