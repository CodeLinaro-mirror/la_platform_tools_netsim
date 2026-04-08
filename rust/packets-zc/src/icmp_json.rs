// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::icmp::IcmpHeader;
use std::collections::BTreeMap;

pub fn to_json(header: &IcmpHeader) -> BTreeMap<String, String> {
    let mut icmp = BTreeMap::new();
    icmp.insert("icmp.type".to_string(), format!("{}", header.icmp_type));
    icmp.insert("icmp.code".to_string(), format!("{}", header.icmp_code));
    icmp.insert("icmp.checksum".to_string(), format!("{:#06x}", header.icmp_checksum.get()));
    icmp
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
        let json_map = to_json(&header);
        let value = serde_json::to_value(json_map).unwrap();
        assert_eq!(value["icmp.type"], "8");
        assert_eq!(value["icmp.code"], "0");
        assert_eq!(value["icmp.checksum"], "0x1234");
    }

    #[test]
    fn test_to_json_string() {
        let header = IcmpHeader {
            icmp_type: 8,
            icmp_code: 0,
            icmp_checksum: U16::new(0x1234),
            rest: [0; 4],
        };
        let json_map = to_json(&header);
        let s = serde_json::to_string(&json_map).unwrap();
        assert!(s.contains("icmp.type"));
    }
}
