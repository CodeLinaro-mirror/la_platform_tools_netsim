// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The pcap::proto_json contains the utility functions
//! that aids translation from protobufs to json
//! This file will be deprecated after protobuf v3 update.

use frontend_proto::common::ChipKind;
use frontend_proto::model::{Pcap as ProtoPcap, State};
use protobuf::well_known_types::Timestamp;

// Will be deprecated once protobuf v3 is imported
fn state_to_string(state: State) -> &'static str {
    match state {
        State::UNKNOWN => "UNKNOWN",
        State::ON => "ON",
        State::OFF => "OFF",
    }
}

// Will be deprecated once protobuf v3 is imported
fn chip_kind_to_string(chip_kind: ChipKind) -> &'static str {
    match chip_kind {
        ChipKind::UNSPECIFIED => "UNSPECIFIED",
        ChipKind::BLUETOOTH => "BLUETOOTH",
        ChipKind::UWB => "UWB",
        ChipKind::WIFI => "WIFI",
    }
}

// Will be deprecated once protobuf v3 is imported
fn write_to_json_str(key: &str, value: String, out: &mut String) {
    if key == "chipKind" || key == "deviceName" || key == "state" {
        out.push_str(format!(r#""{:}": "{:}","#, key, value).as_str());
    } else {
        out.push_str(format!(r#""{:}": {:},"#, key, value).as_str());
    }
}

fn write_timestamp_to_json_str(key: &str, value: &Timestamp, out: &mut String) {
    out.push_str(
        format!(r#""{:}": {{"seconds": {:},"nanos": {:}}},"#, key, value.seconds, value.nanos)
            .as_str(),
    );
}

// Will be deprecated once protobuf v3 is imported
pub fn capture_to_string(proto: &ProtoPcap, out: &mut String) {
    out.push('{');
    write_to_json_str("id", proto.get_id().to_string(), out);
    write_to_json_str("chipKind", chip_kind_to_string(proto.get_chip_kind()).to_string(), out);
    write_to_json_str("chipId", proto.get_chip_id().to_string(), out);
    write_to_json_str("deviceName", proto.get_device_name().to_string(), out);
    write_to_json_str("state", state_to_string(proto.get_state()).to_string(), out);
    write_to_json_str("size", proto.get_size().to_string(), out);
    write_to_json_str("records", proto.get_records().to_string(), out);
    write_timestamp_to_json_str("timestamp", proto.get_timestamp(), out);
    write_to_json_str("valid", proto.get_valid().to_string(), out);
    out.pop();
    out.push_str(r"},");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pcap_to_string() {
        let pcap = ProtoPcap::new();
        let mut out = String::new();
        capture_to_string(&pcap, &mut out);
        let expected = r#"{"id": 0,"chipKind": "UNSPECIFIED","chipId": 0,"deviceName": "","state": "UNKNOWN","size": 0,"records": 0,"timestamp": {"seconds": 0,"nanos": 0},"valid": false},"#;
        assert_eq!(out, expected);
    }

    #[test]
    fn test_modified_pcap_to_string() {
        let mut pcap = ProtoPcap::new();
        let mut out = String::new();
        pcap.id = 1;
        pcap.chip_kind = ChipKind::WIFI;
        pcap.device_name = "sample".to_string();
        capture_to_string(&pcap, &mut out);
        let expected = r#"{"id": 1,"chipKind": "WIFI","chipId": 0,"deviceName": "sample","state": "UNKNOWN","size": 0,"records": 0,"timestamp": {"seconds": 0,"nanos": 0},"valid": false},"#;
        assert_eq!(out, expected);
    }
}
