use crate::icmpv6::Icmpv6Header;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct Icmpv6Packet(BTreeMap<String, String>);

pub fn to_json(icmpv6_packet: &Icmpv6Header) -> Icmpv6Packet {
    let mut icmpv6 = BTreeMap::new();
    icmpv6.insert("icmpv6.type".to_string(), format!("{}", icmpv6_packet.icmpv6_type));
    icmpv6.insert("icmpv6.code".to_string(), format!("{}", icmpv6_packet.icmpv6_code));
    icmpv6.insert(
        "icmpv6.checksum".to_string(),
        format!("{:#06x}", icmpv6_packet.icmpv6_checksum.get()),
    );
    icmpv6.insert("icmpv6.checksum.status".to_string(), "1".to_string());
    Icmpv6Packet(icmpv6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::byteorder::U16;

    #[test]
    fn test_to_json() {
        let header = Icmpv6Header {
            icmpv6_type: 128,
            icmpv6_code: 0,
            icmpv6_checksum: U16::new(0x1234),
            rest: [0; 4],
        };
        let json = to_json(&header);
        let value = serde_json::to_value(&json).unwrap();
        assert_eq!(value["icmpv6.type"], "128");
        assert_eq!(value["icmpv6.code"], "0");
        assert_eq!(value["icmpv6.checksum"], "0x1234");
        assert_eq!(value["icmpv6.checksum.status"], "1");
    }
}
