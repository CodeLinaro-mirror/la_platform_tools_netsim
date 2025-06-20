//! Provides JSON serialization and deserialization for IEEE 802.11 MAC headers.
//!
//! This module defines `serde`-compatible structures that mirror the `zerocopy`
//! IEEE 802.11 structures from the `ieee80211` module. It includes functions
//! for converting between these types and for serializing to/from JSON strings.

use crate::ethernet_json::JsonMacAddr;
use crate::ieee80211::{FrameControl, MacHeader3Addr, SequenceControl};
use crate::ieee80211_util; // For address interpretation
use serde::{Deserialize, Serialize};
use std::fmt;
use zerocopy::byteorder::LittleEndian;
use zerocopy::U16;

/// A custom error type for JSON operations and conversions related to IEEE 802.11.
#[derive(Debug)]
pub enum JsonError {
    SerdeJsonError(serde_json::Error),
    /// Indicates an error during conversion from a JSON representation to a zerocopy type.
    HexParseError(String),
    ConversionError(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::SerdeJsonError(e) => {
                write!(f, "JSON serialization/deserialization error: {}", e)
            }
            JsonError::HexParseError(s) => write!(f, "Hex parsing error: {}", s),
            JsonError::ConversionError(s) => write!(f, "Conversion error: {}", s),
        }
    }
}

impl std::error::Error for JsonError {}

impl From<serde_json::Error> for JsonError {
    fn from(err: serde_json::Error) -> Self {
        JsonError::SerdeJsonError(err)
    }
}

/// A `serde`-compatible, `tshark`-like representation of an IEEE 802.11 Frame Control field.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonFrameControl {
    #[serde(rename = "wlan.fc.type")]
    pub r#type: u8,
    #[serde(rename = "wlan.fc.subtype")]
    pub subtype: u8,
    #[serde(rename = "wlan.fc.tods")]
    pub to_ds: bool,
    #[serde(rename = "wlan.fc.fromds")]
    pub from_ds: bool,
    #[serde(rename = "wlan.fc.morefrag")]
    pub more_fragments: bool,
    #[serde(rename = "wlan.fc.retry")]
    pub retry: bool,
    #[serde(rename = "wlan.fc.pwrmgt")]
    pub power_management: bool,
    #[serde(rename = "wlan.fc.moredata")]
    pub more_data: bool,
    #[serde(rename = "wlan.fc.protected")]
    pub protected_frame: bool,
    #[serde(rename = "wlan.fc.order")]
    pub order: bool,
    #[serde(rename = "wlan.fc.field_hex")] // tshark often uses wlan.fc
    pub field_hex: String,
}

impl From<FrameControl> for JsonFrameControl {
    fn from(fc: FrameControl) -> Self {
        JsonFrameControl {
            r#type: fc.frame_type(),
            subtype: fc.frame_subtype(),
            to_ds: fc.to_ds(),
            from_ds: fc.from_ds(),
            more_fragments: fc.more_fragments(),
            retry: fc.retry(),
            power_management: fc.power_management(),
            more_data: fc.more_data(),
            protected_frame: fc.protected_frame(),
            order: fc.order(),
            field_hex: format!("{:04x}", fc.get()),
        }
    }
}

impl TryFrom<JsonFrameControl> for FrameControl {
    type Error = JsonError;
    fn try_from(json_fc: JsonFrameControl) -> Result<Self, Self::Error> {
        u16::from_str_radix(&json_fc.field_hex, 16).map(FrameControl::new).map_err(|e| {
            JsonError::HexParseError(format!(
                "Invalid FrameControl hex: {}, error: {}",
                json_fc.field_hex, e
            ))
        })
    }
}

/// A `serde`-compatible, `tshark`-like representation of an IEEE 802.11 Sequence Control field.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonSequenceControl {
    #[serde(rename = "wlan.seq")]
    pub sequence_number: u16,
    #[serde(rename = "wlan.frag")]
    pub fragment_number: u8,
    #[serde(rename = "wlan.sc.field_hex")] // tshark might use wlan.sc
    pub field_hex: String,
}

impl From<SequenceControl> for JsonSequenceControl {
    fn from(sc: SequenceControl) -> Self {
        JsonSequenceControl {
            sequence_number: sc.sequence_number(),
            fragment_number: sc.fragment_number(),
            field_hex: format!("{:04x}", sc.get()),
        }
    }
}

impl TryFrom<JsonSequenceControl> for SequenceControl {
    type Error = JsonError;
    fn try_from(json_sc: JsonSequenceControl) -> Result<Self, Self::Error> {
        u16::from_str_radix(&json_sc.field_hex, 16).map(SequenceControl::new).map_err(|e| {
            JsonError::HexParseError(format!(
                "Invalid SequenceControl hex: {}, error: {}",
                json_sc.field_hex, e
            ))
        })
    }
}

/// Inner fields for `JsonMacHeader3Addr`, mimicking `tshark`'s `wlan` object.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonMacHeader3AddrFields {
    #[serde(flatten)]
    pub frame_control: JsonFrameControl,
    #[serde(rename = "wlan.duration")]
    pub duration_id: u16,
    #[serde(rename = "wlan.ra")]
    pub ra: JsonMacAddr,
    #[serde(rename = "wlan.ta")]
    pub ta: JsonMacAddr,
    #[serde(rename = "wlan.da")]
    pub da: JsonMacAddr,
    #[serde(rename = "wlan.sa")]
    pub sa: JsonMacAddr,
    #[serde(rename = "wlan.bssid", skip_serializing_if = "Option::is_none")]
    pub bssid: Option<JsonMacAddr>,
    // Raw addresses for easier reconstruction and direct tshark comparison
    #[serde(rename = "wlan.addr1")]
    pub addr1: JsonMacAddr,
    #[serde(rename = "wlan.addr2")]
    pub addr2: JsonMacAddr,
    #[serde(rename = "wlan.addr3")]
    pub addr3: JsonMacAddr,
    #[serde(flatten)]
    pub sequence_control: JsonSequenceControl,
}

/// A `serde`-compatible, `tshark`-like representation of an IEEE 802.11 MAC header with 3 addresses.
/// This structure creates the top-level "wlan" key.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonMacHeader3Addr {
    pub wlan: JsonMacHeader3AddrFields,
}

impl From<&MacHeader3Addr> for JsonMacHeader3Addr {
    fn from(header: &MacHeader3Addr) -> Self {
        let fields = JsonMacHeader3AddrFields {
            frame_control: header.frame_control.into(),
            duration_id: header.duration_id.get(),
            ra: ieee80211_util::get_receiver_address(header).into(),
            ta: ieee80211_util::get_transmitter_address(header).into(),
            da: ieee80211_util::get_destination_address(header).into(),
            sa: ieee80211_util::get_source_address(header).into(),
            bssid: ieee80211_util::get_bssid(header).map(JsonMacAddr::from),
            addr1: header.addr1.into(),
            addr2: header.addr2.into(),
            addr3: header.addr3.into(),
            sequence_control: header.sequence_control.into(),
        };
        JsonMacHeader3Addr { wlan: fields }
    }
}

impl TryFrom<&JsonMacHeader3Addr> for MacHeader3Addr {
    type Error = crate::ethernet_json::JsonError; // Re-use ethernet's MacAddr parsing error

    fn try_from(json_header: &JsonMacHeader3Addr) -> Result<Self, Self::Error> {
        let wlan_fields = &json_header.wlan;
        Ok(MacHeader3Addr {
            frame_control: FrameControl::try_from(wlan_fields.frame_control.clone())
                .map_err(|e| crate::ethernet_json::JsonError::MacAddrParseError(e.to_string()))?, // Bit of a hack for error type
            duration_id: U16::<LittleEndian>::new(wlan_fields.duration_id),
            addr1: crate::ethernet::MacAddr::try_from(wlan_fields.addr1.clone())?,
            addr2: crate::ethernet::MacAddr::try_from(wlan_fields.addr2.clone())?,
            addr3: crate::ethernet::MacAddr::try_from(wlan_fields.addr3.clone())?,
            sequence_control: SequenceControl::try_from(wlan_fields.sequence_control.clone())
                .map_err(|e| crate::ethernet_json::JsonError::MacAddrParseError(e.to_string()))?,
        })
    }
}

/// Serializes a `MacHeader3Addr` to a JSON string.
pub fn to_json_string(header: &MacHeader3Addr) -> Result<String, JsonError> {
    let json_header = JsonMacHeader3Addr::from(header);
    serde_json::to_string_pretty(&json_header).map_err(JsonError::from)
}

/// Deserializes a `MacHeader3Addr` from a JSON string.
pub fn from_json_string(json_str: &str) -> Result<MacHeader3Addr, JsonError> {
    let json_header: JsonMacHeader3Addr = serde_json::from_str(json_str)?;
    MacHeader3Addr::try_from(&json_header).map_err(|e| JsonError::ConversionError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ethernet::MacAddr as EthMacAddr; // Alias to avoid confusion
    use crate::ieee80211::frame_type;

    #[test]
    fn test_json_frame_control_conversion() {
        let fc_orig = FrameControl::new(0x0108); // Data, ToDS
        let json_fc = JsonFrameControl::from(fc_orig);
        assert_eq!(json_fc.r#type, frame_type::DATA);
        assert_eq!(json_fc.to_ds, true);
        assert_eq!(json_fc.field_hex, "0108");

        let fc_back = FrameControl::try_from(json_fc).unwrap();
        assert_eq!(fc_back.get(), fc_orig.get());

        let invalid_json_fc = JsonFrameControl {
            r#type: 0,
            subtype: 0,
            to_ds: false,
            from_ds: false,
            more_fragments: false,
            retry: false,
            power_management: false,
            more_data: false,
            protected_frame: false,
            order: false,
            field_hex: "invalid".to_string(),
        };
        assert!(FrameControl::try_from(invalid_json_fc).is_err());
    }

    #[test]
    fn test_json_sequence_control_conversion() {
        let sc_orig = SequenceControl::new(0x1234); // Seq = 0x123, Frag = 4
        let json_sc = JsonSequenceControl::from(sc_orig);
        assert_eq!(json_sc.sequence_number, 0x123);
        assert_eq!(json_sc.fragment_number, 4);
        assert_eq!(json_sc.field_hex, "1234");

        let sc_back = SequenceControl::try_from(json_sc).unwrap();
        assert_eq!(sc_back.get(), sc_orig.get());

        let invalid_json_sc = JsonSequenceControl {
            sequence_number: 0,
            fragment_number: 0,
            field_hex: "invalid".to_string(),
        };
        assert!(SequenceControl::try_from(invalid_json_sc).is_err());
    }

    #[test]
    fn test_bssid_serialization_wds() {
        // WDS frame (ToDS=1, FromDS=1), get_bssid returns None
        let header_wds = MacHeader3Addr {
            frame_control: FrameControl::new(0x0308), // Data, ToDS, FromDS
            duration_id: U16::new(0),
            addr1: EthMacAddr::new([1; 6]),
            addr2: EthMacAddr::new([2; 6]),
            addr3: EthMacAddr::new([3; 6]),
            sequence_control: SequenceControl::new(0),
        };
        let json_string = to_json_string(&header_wds).unwrap();
        assert!(!json_string.contains("wlan.bssid"));
    }
    #[test]
    fn test_mac_header_3_addr_json_serialization_deserialization() {
        let original_header = MacHeader3Addr {
            frame_control: FrameControl::new(0x0108), // Data, ToDS
            duration_id: U16::new(100),
            addr1: EthMacAddr::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
            addr2: EthMacAddr::new([0x11, 0x12, 0x13, 0x14, 0x15, 0x16]),
            addr3: EthMacAddr::new([0x21, 0x22, 0x23, 0x24, 0x25, 0x26]),
            sequence_control: SequenceControl::new(0x00A1), // Seq=10, Frag=1
        };

        let json_string = to_json_string(&original_header).unwrap();

        let parsed_json_struct: JsonMacHeader3Addr = serde_json::from_str(&json_string).unwrap();

        // Verify some key fields in the parsed JSON structure
        assert_eq!(parsed_json_struct.wlan.frame_control.r#type, frame_type::DATA);
        assert_eq!(parsed_json_struct.wlan.frame_control.to_ds, true);
        assert_eq!(parsed_json_struct.wlan.frame_control.field_hex, "0108");
        assert_eq!(parsed_json_struct.wlan.duration_id, 100);
        assert_eq!(parsed_json_struct.wlan.addr1.0, "01:02:03:04:05:06");
        assert_eq!(parsed_json_struct.wlan.sequence_control.sequence_number, 10);
        assert_eq!(parsed_json_struct.wlan.sequence_control.fragment_number, 1);
        assert_eq!(parsed_json_struct.wlan.sequence_control.field_hex, "00a1"); // Note: hex crate might output lowercase

        // Check RA/TA/DA/SA/BSSID based on ToDS=1, FromDS=0
        // Addr1: BSSID (RA)
        // Addr2: SA (TA)
        // Addr3: DA
        assert_eq!(parsed_json_struct.wlan.ra.0, "01:02:03:04:05:06");
        assert_eq!(parsed_json_struct.wlan.ta.0, "11:12:13:14:15:16");
        assert_eq!(parsed_json_struct.wlan.da.0, "21:22:23:24:25:26");
        assert_eq!(parsed_json_struct.wlan.sa.0, "11:12:13:14:15:16");
        assert_eq!(parsed_json_struct.wlan.bssid.unwrap().0, "01:02:03:04:05:06");

        let deserialized_header = from_json_string(&json_string).unwrap();

        assert_eq!(deserialized_header.frame_control.get(), original_header.frame_control.get());
        assert_eq!(deserialized_header.duration_id.get(), original_header.duration_id.get());
        assert_eq!(deserialized_header.addr1, original_header.addr1);
        assert_eq!(deserialized_header.addr2, original_header.addr2);
        assert_eq!(deserialized_header.addr3, original_header.addr3);
        assert_eq!(
            deserialized_header.sequence_control.get(),
            original_header.sequence_control.get()
        );

        // Test with a different FrameControl for address roles
        let original_header_ibss = MacHeader3Addr {
            frame_control: FrameControl::new(0x0008), // Data, ToDS=0, FromDS=0 (IBSS)
            duration_id: U16::new(50),
            addr1: EthMacAddr::new([0xAA; 6]),              // DA
            addr2: EthMacAddr::new([0xBB; 6]),              // SA
            addr3: EthMacAddr::new([0xCC; 6]),              // BSSID
            sequence_control: SequenceControl::new(0x0010), // Seq=1, Frag=0
        };
        let json_string_ibss = to_json_string(&original_header_ibss).unwrap();
        let parsed_json_ibss: JsonMacHeader3Addr = serde_json::from_str(&json_string_ibss).unwrap();

        assert_eq!(parsed_json_ibss.wlan.da.0, "AA:AA:AA:AA:AA:AA");
        assert_eq!(parsed_json_ibss.wlan.sa.0, "BB:BB:BB:BB:BB:BB");
        assert_eq!(parsed_json_ibss.wlan.bssid.unwrap().0, "CC:CC:CC:CC:CC:CC");
        assert_eq!(parsed_json_ibss.wlan.ra.0, "AA:AA:AA:AA:AA:AA");
        assert_eq!(parsed_json_ibss.wlan.ta.0, "BB:BB:BB:BB:BB:BB");
    }

    #[test]
    fn test_invalid_mac_in_header_json_deserialize() {
        let json_str = r#"{
            "wlan": {
                "wlan.fc.type": 2, "wlan.fc.subtype": 0, "wlan.fc.tods": false, "wlan.fc.fromds": false,
                "wlan.fc.morefrag": false, "wlan.fc.retry": false, "wlan.fc.pwrmgt": false,
                "wlan.fc.moredata": false, "wlan.fc.protected": false, "wlan.fc.order": false,
                "wlan.fc.field_hex": "0008",
                "wlan.duration": 100,
                "wlan.ra": "01:02:03:04:05:06",
                "wlan.ta": "INVALID:MAC:ADDRESS",
                "wlan.da": "01:02:03:04:05:06",
                "wlan.sa": "INVALID:MAC:ADDRESS",
                "wlan.addr1": "01:02:03:04:05:06",
                "wlan.addr2": "INVALID:MAC:ADDRESS",
                "wlan.addr3": "21:22:23:24:25:26",
                "wlan.seq": 0, "wlan.frag": 0,
                "wlan.sc.field_hex": "0000"
            }
        }"#;
        assert!(from_json_string(json_str).is_err());
    }

    #[test]
    fn test_deserialize_missing_optional_bssid() {
        // JSON string deliberately missing wlan.bssid
        let json_str = r#"{
            "wlan": {
                "wlan.fc.type": 0, "wlan.fc.subtype": 8, "wlan.fc.tods": false, "wlan.fc.fromds": false,
                "wlan.fc.morefrag": false, "wlan.fc.retry": false, "wlan.fc.pwrmgt": false,
                "wlan.fc.moredata": false, "wlan.fc.protected": false, "wlan.fc.order": false,
                "wlan.fc.field_hex": "0080",
                "wlan.duration": 0,
                "wlan.ra": "ff:ff:ff:ff:ff:ff",
                "wlan.ta": "00:11:22:33:44:55",
                "wlan.da": "ff:ff:ff:ff:ff:ff",
                "wlan.sa": "00:11:22:33:44:55",
                "wlan.addr1": "ff:ff:ff:ff:ff:ff",
                "wlan.addr2": "00:11:22:33:44:55",
                "wlan.addr3": "00:11:22:33:44:55",
                "wlan.seq": 1, "wlan.frag": 0,
                "wlan.sc.field_hex": "0010"
            }
        }"#;
        let result = from_json_string(json_str);
        assert!(result.is_ok(), "Deserialization failed: {:?}", result.err());
    }
}
