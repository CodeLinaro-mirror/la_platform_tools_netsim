// Copyright 2024 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS-IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::icmp::IcmpHeader;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub struct IcmpHeaderJson<'a> {
    header: &'a IcmpHeader,
}

impl<'a> Serialize for IcmpHeaderJson<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("IcmpHeader", 3)?;
        s.serialize_field("type", &self.header.icmp_type)?;
        s.serialize_field("code", &self.header.icmp_code)?;
        s.serialize_field("checksum", &self.header.icmp_checksum.get())?;
        s.end()
    }
}

pub fn to_json(icmp_header: &IcmpHeader) -> IcmpHeaderJson {
    IcmpHeaderJson { header: icmp_header }
}

pub fn to_json_string(icmp_header: &IcmpHeader) -> Result<String, serde_json::Error> {
    let json = IcmpHeaderJson { header: icmp_header };
    serde_json::to_string(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::byteorder::U16;

    #[test]
    fn test_to_json() {
        let header = IcmpHeader {
            icmp_type: 8,
            icmp_code: 0,
            icmp_checksum: U16::new(0x1234),
            rest: [0; 4],
        };
        let json = to_json(&header);
        let value = serde_json::to_value(&json).unwrap();
        assert_eq!(value["type"], 8);
        assert_eq!(value["code"], 0);
        assert_eq!(value["checksum"], 0x1234);
    }

    #[test]
    fn test_to_json_string() {
        let header = IcmpHeader {
            icmp_type: 0,
            icmp_code: 0,
            icmp_checksum: U16::new(0x5678),
            rest: [0; 4],
        };
        let json_string = to_json_string(&header).unwrap();
        assert_eq!(json_string, r#"{"type":0,"code":0,"checksum":22136}"#);
    }
}
