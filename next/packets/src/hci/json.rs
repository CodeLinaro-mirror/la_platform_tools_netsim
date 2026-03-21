// Copyright 2026 The Android Open Source Project

use serde::{Deserialize, Serialize};

use crate::{
    hci::events::{EventCode, HciEvent, LeAdvertisingReport, LeMetaEvent, SubeventCode},
    utils::json as json_common,
};

/// Top-level JSON representation of an HCI Event.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonHciEvent {
    #[serde(rename = "bluetooth.hci_event.code")]
    pub event_code: u8,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub le_meta: Option<JsonLeMetaEvent>,
}

/// JSON representation of an LE Meta Event.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonLeMetaEvent {
    #[serde(rename = "bluetooth.hci_le.subevent_code")]
    pub subevent_code: u8,
    #[serde(
        rename = "bluetooth.hci_le.advertising_report.num_reports",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_reports: Option<u8>,
    #[serde(
        rename = "bluetooth.hci_le.advertising_report",
        skip_serializing_if = "Option::is_none"
    )]
    pub reports: Option<Vec<JsonLeAdvertisingReport>>,
}

/// JSON representation of an LE Advertising Report.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JsonLeAdvertisingReport {
    #[serde(rename = "bluetooth.hci_le.advertising_report.event_type")]
    pub event_type: u8,
    #[serde(rename = "bluetooth.hci_le.advertising_report.address_type")]
    pub address_type: u8,
    #[serde(rename = "bluetooth.addr")]
    pub address: String,
    #[serde(rename = "bluetooth.hci_le.advertising_report.data_length")]
    pub data_length: u8,
    #[serde(rename = "bluetooth.hci_le.advertising_report.data")]
    pub data: String,
    #[serde(rename = "bluetooth.hci_le.advertising_report.rssi")]
    pub rssi: i8,
}

impl From<&HciEvent<'_>> for JsonHciEvent {
    fn from(event: &HciEvent<'_>) -> Self {
        match event {
            HciEvent::LeMetaEvent(le_event) => JsonHciEvent {
                event_code: EventCode::LE_META_EVENT.0,
                le_meta: Some(JsonLeMetaEvent::from(le_event)),
            },
            HciEvent::CommandComplete { header: _, .. } => {
                JsonHciEvent { event_code: EventCode::COMMAND_COMPLETE.0, le_meta: None }
            }
            HciEvent::CommandStatus(_status) => {
                JsonHciEvent { event_code: EventCode::COMMAND_STATUS.0, le_meta: None }
            }
            HciEvent::Unknown { event_code, .. } => {
                JsonHciEvent { event_code: event_code.0, le_meta: None }
            }
        }
    }
}

impl From<&LeMetaEvent<'_>> for JsonLeMetaEvent {
    fn from(event: &LeMetaEvent<'_>) -> Self {
        match event {
            LeMetaEvent::LeAdvertisingReport(reports) => JsonLeMetaEvent {
                subevent_code: SubeventCode::LE_ADVERTISING_REPORT.0,
                num_reports: Some(reports.len() as u8),
                reports: Some(reports.iter().map(JsonLeAdvertisingReport::from).collect()),
            },
            LeMetaEvent::Unknown { subevent_code, .. } => {
                JsonLeMetaEvent { subevent_code: subevent_code.0, num_reports: None, reports: None }
            }
        }
    }
}

impl From<&LeAdvertisingReport<'_>> for JsonLeAdvertisingReport {
    fn from(report: &LeAdvertisingReport<'_>) -> Self {
        JsonLeAdvertisingReport {
            event_type: report.event_type,
            address_type: report.address_type,
            address: report.address.to_string(),
            data_length: report.data_length,
            data: hex::encode(report.data),
            rssi: report.rssi,
        }
    }
}

/// Serializes an `HciEvent` to a JSON Value (tshark style).
pub fn to_json(event: &HciEvent, packet_len: usize) -> serde_json::Value {
    let json_event = JsonHciEvent::from(event);
    let hci_val = serde_json::to_value(json_event).unwrap();

    let mut layers = serde_json::Map::new();
    if let Some(obj) = hci_val.as_object() {
        layers.insert("bluetooth".to_string(), serde_json::Value::Object(obj.clone()));
    }

    json_common::build_packet_json(serde_json::Value::Object(layers), packet_len, "bluetooth")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hci::events::parse_hci_event;

    #[test]
    fn test_le_advertising_report_to_json() {
        let data = [
            0x3E, // Event Code: LE Meta Event
            12,   // Parameter Total Length
            0x02, // Subevent Code: LE Advertising Report
            0x01, // Num Reports
            0x00, // Event Type
            0x00, // Address Type
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, // Address
            0x01, // Data Length
            0x11, // Data
            0xCE, // RSSI (-50)
        ];
        let event = parse_hci_event(&data).unwrap();
        let json = to_json(&event, 14);
        let bluetooth = &json[0]["_source"]["layers"]["bluetooth"];
        assert_eq!(bluetooth["bluetooth.hci_event.code"], 0x3E);
        assert_eq!(bluetooth["bluetooth.hci_le.subevent_code"], 0x02);
        assert_eq!(bluetooth["bluetooth.hci_le.advertising_report.num_reports"], 1);
        let reports = &bluetooth["bluetooth.hci_le.advertising_report"];
        assert_eq!(reports[0]["bluetooth.addr"], "FF:EE:DD:CC:BB:AA");
        assert_eq!(reports[0]["bluetooth.hci_le.advertising_report.rssi"], -50);
        assert_eq!(reports[0]["bluetooth.hci_le.advertising_report.data"], "11");
    }
}
