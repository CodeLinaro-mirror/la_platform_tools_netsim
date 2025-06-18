//! Provides JSON serialization and deserialization for Ethernet frames.
//!
//! This module defines `serde`-compatible structures that mirror the `zerocopy`
//! Ethernet structures from the `ethernet` module. It includes functions
//! for converting between these types and for serializing to/from JSON strings.

use crate::ethernet::{EthernetFrame, MacAddr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use zerocopy::U16;

/// A custom error type for JSON operations and conversions.
#[derive(Debug)]
pub enum JsonError {
    SerdeJsonError(serde_json::Error),
    MacAddrParseError(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::SerdeJsonError(e) => {
                write!(f, "JSON serialization/deserialization error: {}", e)
            }
            JsonError::MacAddrParseError(s) => write!(f, "MAC address parsing error: {}", s),
        }
    }
}

impl std::error::Error for JsonError {}

impl From<serde_json::Error> for JsonError {
    fn from(err: serde_json::Error) -> Self {
        JsonError::SerdeJsonError(err)
    }
}

/// A `serde`-compatible representation of a MAC address.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonMacAddr(pub String);

impl From<MacAddr> for JsonMacAddr {
    fn from(mac: MacAddr) -> Self {
        JsonMacAddr(mac.to_string())
    }
}

impl TryFrom<JsonMacAddr> for MacAddr {
    type Error = JsonError;

    fn try_from(json_mac: JsonMacAddr) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = json_mac.0.split(':').collect();
        if parts.len() != 6 {
            return Err(JsonError::MacAddrParseError(format!(
                "Invalid MAC address format: expected 6 parts, got {}",
                json_mac.0
            )));
        }
        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16).map_err(|_| {
                JsonError::MacAddrParseError(format!("Invalid hex byte in MAC address: {}", part))
            })?;
        }
        Ok(MacAddr { bytes })
    }
}

/// Inner fields for `JsonEthernetFrame`, mimicking `tshark`'s `eth` object.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonEthernetFrameFields {
    #[serde(rename = "eth.dst")]
    pub dst_addr: JsonMacAddr,
    #[serde(rename = "eth.src")]
    pub src_addr: JsonMacAddr,
    #[serde(rename = "eth.type")]
    pub ethertype: u16,
    // Potentially add other interpreted fields like eth.dst.ig, eth.dst.lg if needed
}

/// A `serde`-compatible, `tshark`-like representation of an Ethernet frame.
/// This structure creates the top-level "eth" key.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonEthernetFrame {
    // Using a HashMap to allow for a single "eth" key, similar to tshark's root object structure.
    // Alternatively, you could define a struct with a single field `pub eth: JsonEthernetFrameFields`.
    // Using HashMap for flexibility if other top-level keys (like "frame", "ip") were to be added.
    #[serde(flatten)]
    pub layers: HashMap<String, JsonEthernetFrameFields>,
}

impl From<&EthernetFrame> for JsonEthernetFrame {
    fn from(frame: &EthernetFrame) -> Self {
        let fields = JsonEthernetFrameFields {
            dst_addr: JsonMacAddr::from(frame.dst_addr),
            src_addr: JsonMacAddr::from(frame.src_addr),
            ethertype: frame.ethertype.get(), // .get() converts from NetworkEndian to host u16
        };
        let mut layers = HashMap::new();
        layers.insert("eth".to_string(), fields);
        JsonEthernetFrame { layers }
    }
}

impl TryFrom<&JsonEthernetFrame> for EthernetFrame {
    type Error = JsonError;

    fn try_from(json_frame: &JsonEthernetFrame) -> Result<Self, Self::Error> {
        let fields = json_frame.layers.get("eth").ok_or_else(|| {
            JsonError::MacAddrParseError("Missing 'eth' layer in JSON".to_string())
        })?;

        Ok(EthernetFrame {
            dst_addr: MacAddr::try_from(fields.dst_addr.clone())?,
            src_addr: MacAddr::try_from(fields.src_addr.clone())?,
            ethertype: U16::new(fields.ethertype),
        })
    }
}

/// Serializes an `EthernetFrame` to a JSON string.
pub fn to_json_string(frame: &EthernetFrame) -> Result<String, JsonError> {
    let json_frame = JsonEthernetFrame::from(frame);
    serde_json::to_string_pretty(&json_frame).map_err(JsonError::from)
}

/// Deserializes an `EthernetFrame` from a JSON string.
pub fn from_json_string(json_str: &str) -> Result<EthernetFrame, JsonError> {
    let json_frame: JsonEthernetFrame = serde_json::from_str(json_str).map_err(JsonError::from)?;
    EthernetFrame::try_from(&json_frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ethernet::ether_type;
    use zerocopy::byteorder::NetworkEndian; // Added this import

    #[test]
    fn test_mac_addr_json_conversion() {
        let mac = MacAddr { bytes: [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02] };
        let json_mac = JsonMacAddr::from(mac);
        assert_eq!(json_mac.0, "DE:AD:BE:EF:01:02");

        let parsed_mac = MacAddr::try_from(json_mac).unwrap();
        assert_eq!(parsed_mac, mac);
    }

    #[test]
    fn test_ethernet_frame_json_serialization_deserialization() {
        let original_frame = EthernetFrame {
            dst_addr: MacAddr { bytes: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06] },
            src_addr: MacAddr { bytes: [0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C] },
            ethertype: U16::<NetworkEndian>::new(ether_type::IPV4),
        };

        let json_string = to_json_string(&original_frame).unwrap();

        // We expect the JSON to look something like:
        // { "eth": { "eth.dst": "...", "eth.src": "...", "eth.type": ... } }
        let parsed_value: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        let eth_layer = parsed_value.get("eth").unwrap().as_object().unwrap();

        assert_eq!(eth_layer.get("eth.dst").unwrap().as_str().unwrap(), "01:02:03:04:05:06");
        assert_eq!(eth_layer.get("eth.src").unwrap().as_str().unwrap(), "07:08:09:0A:0B:0C");
        assert_eq!(eth_layer.get("eth.type").unwrap().as_u64().unwrap(), ether_type::IPV4 as u64);

        let deserialized_frame = from_json_string(&json_string).unwrap();

        assert_eq!(deserialized_frame.dst_addr, original_frame.dst_addr);
        assert_eq!(deserialized_frame.src_addr, original_frame.src_addr);
        assert_eq!(deserialized_frame.ethertype.get(), original_frame.ethertype.get());
    }

    #[test]
    fn test_invalid_mac_json_deserialize() {
        let json_mac_invalid = JsonMacAddr("INVALID:MAC".to_string());
        assert!(MacAddr::try_from(json_mac_invalid).is_err());

        let json_mac_invalid_hex = JsonMacAddr("DE:AD:BE:EF:01:ZZ".to_string());
        assert!(MacAddr::try_from(json_mac_invalid_hex).is_err());

        // Test full frame deserialization with invalid MAC
        let invalid_json_frame_str = r#"{
            "eth": {
                "eth.dst": "INVALID:MAC",
                "eth.src": "07:08:09:0A:0B:0C",
                "eth.type": 2048
            }
        }"#;
        assert!(from_json_string(invalid_json_frame_str).is_err());
    }
}
