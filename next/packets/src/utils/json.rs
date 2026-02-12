// Copyright 2025 The Android Open Source Project

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub fn build_packet_json(layers: Value, packet_len: usize, protocols: &str) -> Value {
    let now = SystemTime::now();
    let epoch_secs = now.duration_since(UNIX_EPOCH).unwrap().as_secs_f64();
    let frame = serde_json::json!({
        "frame.time_epoch": format!("{:.9}", epoch_secs),
        "frame.len": packet_len,
        "frame.cap_len": packet_len, // Assuming no truncation for now
        "frame.protocols": protocols
    });
    let mut source = serde_json::json!({ "layers": layers });
    let layers_mut = source["layers"].as_object_mut().unwrap();
    layers_mut.insert("frame".to_string(), frame);
    serde_json::json!([{
        "_index": "packets-2025-06-14",
        "_type": "doc",
        "_score": null,
        "_source": source
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_packet_json() {
        let layers = serde_json::json!({
            "eth": {
                "eth.dst": "ff:ff:ff:ff:ff:ff",
                "eth.src": "00:00:00:00:00:00"
            }
        });
        let result = build_packet_json(layers, 60, "eth:ip");
        let source = &result[0]["_source"];
        assert!(source["layers"]["frame"]["frame.time_epoch"].is_string());
        assert_eq!(source["layers"]["eth"]["eth.dst"], "ff:ff:ff:ff:ff:ff");
    }
}
