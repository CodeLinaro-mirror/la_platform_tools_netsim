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

//! ieee80211 frames

// TODO: only allow the warnings for the included code
#![allow(clippy::all)]
#![allow(missing_docs)]
#![allow(unused)]
include!(concat!(env!("OUT_DIR"), "/ieee80211_packets.rs"));

use anyhow::anyhow;

/// A Ieee80211 MAC address

impl MacAddress {
    pub fn to_vec(&self) -> [u8; 6] {
        u64::to_le_bytes(self.0)[0..6].try_into().expect("slice with incorrect length")
    }
}

// TODO: Add unit tests.
impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = u64::to_le_bytes(self.0);
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
        )
    }
}

impl fmt::Display for Ieee80211 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ds: {}, src: {}, dst: {}}}",
            self.get_ds(),
            self.get_source(),
            self.get_destination()
        )
    }
}

impl From<&[u8; 6]> for MacAddress {
    fn from(bytes: &[u8; 6]) -> Self {
        Self(u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], 0, 0]))
    }
}

impl From<MacAddress> for [u8; 6] {
    fn from(MacAddress(addr): MacAddress) -> Self {
        let bytes = u64::to_le_bytes(addr);
        bytes[0..6].try_into().unwrap()
    }
}

impl MacAddress {
    pub fn is_multicast(&self) -> bool {
        let addr = u64::to_le_bytes(self.0);
        (addr[0] & 0x1) == 1
    }

    pub fn is_broadcast(&self) -> bool {
        self.0 == u64::MAX
    }
}

impl Ieee80211 {
    // Frame has addr4 field
    pub fn has_a4(&self) -> bool {
        self.to_ds == 1 || self.from_ds == 1
    }

    pub fn is_to_ap(&self) -> bool {
        self.to_ds == 1 && self.from_ds == 0
    }

    // Frame type is management
    pub fn is_mgmt(&self) -> bool {
        self.ftype == FrameType::Mgmt
    }

    // Frame type is data
    pub fn is_data(&self) -> bool {
        self.ftype == FrameType::Data
    }

    // Frame is probe request
    pub fn is_probe_req(&self) -> bool {
        self.ftype == FrameType::Ctl && self.stype == (ManagementSubType::ProbeReq as u8)
    }

    pub fn get_ds(&self) -> String {
        match self.specialize().unwrap() {
            Ieee80211Child::Ieee80211ToAp(hdr) => "ToAp",
            Ieee80211Child::Ieee80211FromAp(hdr) => "FromAp",
            Ieee80211Child::Ieee80211Ibss(hdr) => "Ibss",
            Ieee80211Child::Ieee80211Wds(hdr) => "Wds",
            _ => panic!("unexpected specialized header"),
        }
        .to_string()
    }

    pub fn get_source(&self) -> MacAddress {
        match self.specialize().unwrap() {
            Ieee80211Child::Ieee80211ToAp(hdr) => hdr.source,
            Ieee80211Child::Ieee80211FromAp(hdr) => hdr.source,
            Ieee80211Child::Ieee80211Ibss(hdr) => hdr.source,
            Ieee80211Child::Ieee80211Wds(hdr) => hdr.source,
            _ => panic!("unexpected specialized header"),
        }
    }

    /// Ieee80211 packets have 3-4 addresses in different positions based
    /// on the FromDS and ToDS flags. This function gets the destination
    /// address depending on the FromDS+ToDS packet subtypes.
    pub fn get_destination(&self) -> MacAddress {
        match self.specialize().unwrap() {
            Ieee80211Child::Ieee80211ToAp(hdr) => hdr.destination,
            Ieee80211Child::Ieee80211FromAp(hdr) => hdr.destination,
            Ieee80211Child::Ieee80211Ibss(hdr) => hdr.destination,
            Ieee80211Child::Ieee80211Wds(hdr) => hdr.destination,
            _ => panic!("unexpected specialized header"),
        }
    }

    pub fn get_bssid(&self) -> Option<MacAddress> {
        match self.specialize().unwrap() {
            Ieee80211Child::Ieee80211ToAp(hdr) => Some(hdr.bssid),
            Ieee80211Child::Ieee80211FromAp(hdr) => Some(hdr.bssid),
            Ieee80211Child::Ieee80211Ibss(hdr) => Some(hdr.bssid),
            Ieee80211Child::Ieee80211Wds(hdr) => None,
            _ => panic!("unexpected specialized header"),
        }
    }

    pub fn with_address(
        &self,
        source: Option<MacAddress>,
        destination: Option<MacAddress>,
    ) -> Ieee80211 {
        match self.specialize().unwrap() {
            Ieee80211Child::Ieee80211ToAp(frame) => {
                frame.with_address(source, destination).try_into().unwrap()
            }
            Ieee80211Child::Ieee80211FromAp(frame) => {
                frame.with_address(source, destination).try_into().unwrap()
            }
            Ieee80211Child::Ieee80211Ibss(frame) => {
                frame.with_address(source, destination).try_into().unwrap()
            }
            Ieee80211Child::Ieee80211Wds(frame) => {
                frame.with_address(source, destination).try_into().unwrap()
            }
            _ => panic!("Unknown Ieee80211Child type"),
        }
    }

    /// Covert Ieee80211ToAp to Ieee80211FromAp packet.
    pub fn into_from_ap(&self) -> anyhow::Result<Ieee80211FromAp> {
        match self.specialize().unwrap() {
            Ieee80211Child::Ieee80211ToAp(frame_to_ap) => {
                // Flip from_ap and to_ap bits.
                // TODO: Investigate if there is a way to copy frame_control flags at once.
                // The header struct only has 7 fields, not 15. Most fields come from le16 frame_control.
                Ok(Ieee80211FromAp {
                    duration_id: frame_to_ap.duration_id,
                    ftype: frame_to_ap.ftype,
                    more_data: frame_to_ap.more_data,
                    more_frags: frame_to_ap.more_frags,
                    order: frame_to_ap.order,
                    pm: frame_to_ap.pm,
                    protected: frame_to_ap.protected,
                    retry: frame_to_ap.retry,
                    stype: frame_to_ap.stype,
                    version: frame_to_ap.version,
                    bssid: frame_to_ap.bssid,
                    source: frame_to_ap.source,
                    destination: frame_to_ap.destination,
                    seq_ctrl: frame_to_ap.seq_ctrl,
                    payload: frame_to_ap.payload.to_vec(),
                })
            }
            _ => Err(anyhow!(
                "Invalid Ieee80211Child packet. from_ds: {}, to_ds: {}",
                self.from_ds,
                self.to_ds
            )),
        }
    }
}

impl Ieee80211FromAp {
    pub fn with_address(
        &self,
        source: Option<MacAddress>,
        destination: Option<MacAddress>,
    ) -> Ieee80211FromAp {
        Ieee80211FromAp {
            source: source.unwrap_or(self.source),
            destination: destination.unwrap_or(self.destination),
            ..self.clone()
        }
    }
}

impl Ieee80211ToAp {
    pub fn with_address(
        &self,
        source: Option<MacAddress>,
        destination: Option<MacAddress>,
    ) -> Ieee80211ToAp {
        Ieee80211ToAp {
            source: source.unwrap_or(self.source),
            destination: destination.unwrap_or(self.destination),
            ..self.clone()
        }
    }
}

impl Ieee80211Ibss {
    pub fn with_address(
        &self,
        source: Option<MacAddress>,
        destination: Option<MacAddress>,
    ) -> Ieee80211Ibss {
        Ieee80211Ibss {
            source: source.unwrap_or(self.source),
            destination: destination.unwrap_or(self.destination),
            ..self.clone()
        }
    }
}

impl Ieee80211Wds {
    pub fn with_address(
        &self,
        source: Option<MacAddress>,
        destination: Option<MacAddress>,
    ) -> Ieee80211Wds {
        Ieee80211Wds {
            source: source.unwrap_or(self.source),
            destination: destination.unwrap_or(self.destination),
            ..self.clone()
        }
    }
}

pub fn parse_mac_address(s: &str) -> Option<MacAddress> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut bytes = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        match u8::from_str_radix(part, 16) {
            Ok(n) => bytes[i] = n,
            Err(e) => return None,
        }
    }
    Some(MacAddress::from(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mad_address_to_vec() {
        let mac_address: MacAddress = parse_mac_address("00:0b:85:71:20:ce").unwrap();
        let mac_address_bytes = mac_address.to_vec();
        let reconstructed_mac_address = MacAddress::from(&mac_address_bytes);
        assert_eq!(mac_address, reconstructed_mac_address);
    }

    // These tests use the packets available here
    // https://community.cisco.com/t5/wireless-mobility-knowledge-base/802-11-frames-a-starter-guide-to-learn-wireless-sniffer-traces/ta-p/3110019

    #[test]
    fn test_frame_qos() {
        let frame: Vec<u8> = vec![
            0x88, 0x02, 0x2c, 0x00, 0x00, 0x13, 0xe8, 0xeb, 0xd6, 0x03, 0x00, 0x0b, 0x85, 0x71,
            0x20, 0xce, 0x00, 0x0b, 0x85, 0x71, 0x20, 0xce, 0x00, 0x26, 0x00, 0x00,
        ];
        let hdr = Ieee80211::decode_full(&frame).unwrap();
        assert!(hdr.is_data());
        assert_eq!(hdr.stype, DataSubType::Qos as u8);
        assert_eq!(hdr.from_ds, 1);
        assert_eq!(hdr.to_ds, 0);
        assert_eq!(hdr.duration_id, 44);
        // Source address: Cisco_71:20:ce (00:0b:85:71:20:ce)
        let a = format!("{}", hdr.get_source());
        let b = format!("{}", parse_mac_address("00:0b:85:71:20:ce").unwrap());
        assert_eq!(a, b);
    }

    #[test]
    fn test_is_multicast() {
        // Multicast MAC address: 01:00:5E:00:00:FB
        let mdns_mac_address = parse_mac_address("01:00:5e:00:00:fb").unwrap();
        assert!(mdns_mac_address.is_multicast());
        // Broadcast MAC address: ff:ff:ff:ff:ff:ff
        let broadcast_mac_address = parse_mac_address("ff:ff:ff:ff:ff:ff").unwrap();
        assert!(broadcast_mac_address.is_multicast());
        // Source address: Cisco_71:20:ce (00:0b:85:71:20:ce)
        let non_mdns_mac_address = parse_mac_address("00:0b:85:71:20:ce").unwrap();
        assert!(!non_mdns_mac_address.is_multicast());
    }

    fn test_is_broadcast() {
        // Multicast MAC address: 01:00:5E:00:00:FB
        let mdns_mac_address = parse_mac_address("01:00:5e:00:00:fb").unwrap();
        assert!(!mdns_mac_address.is_broadcast());
        // Broadcast MAC address: ff:ff:ff:ff:ff:ff
        let broadcast_mac_address = parse_mac_address("ff:ff:ff:ff:ff:ff").unwrap();
        assert!(broadcast_mac_address.is_broadcast());
        // Source address: Cisco_71:20:ce (00:0b:85:71:20:ce)
        let non_mdns_mac_address = parse_mac_address("00:0b:85:71:20:ce").unwrap();
        assert!(!non_mdns_mac_address.is_broadcast());
    }

    fn create_test_from_ap_ieee80211(
        source: MacAddress,
        destination: MacAddress,
        bssid: MacAddress,
    ) -> Ieee80211 {
        Ieee80211FromAp {
            duration_id: 0,
            ftype: FrameType::Mgmt,
            more_data: 0,
            more_frags: 0,
            order: 0,
            pm: 0,
            protected: 0,
            retry: 0,
            stype: 0,
            version: 0,
            bssid,
            source,
            destination,
            seq_ctrl: 0,
            payload: Vec::new(),
        }
        .try_into()
        .unwrap()
    }

    fn create_test_ibss_ieee80211(
        source: MacAddress,
        destination: MacAddress,
        bssid: MacAddress,
    ) -> Ieee80211 {
        Ieee80211Ibss {
            duration_id: 0,
            ftype: FrameType::Mgmt,
            more_data: 0,
            more_frags: 0,
            order: 0,
            pm: 0,
            protected: 0,
            retry: 0,
            stype: 0,
            version: 0,
            bssid,
            source,
            destination,
            seq_ctrl: 0,
            payload: Vec::new(),
        }
        .try_into()
        .unwrap()
    }

    fn create_test_to_ap_ieee80211(
        source: MacAddress,
        destination: MacAddress,
        bssid: MacAddress,
    ) -> Ieee80211 {
        Ieee80211ToAp {
            duration_id: 0,
            ftype: FrameType::Mgmt,
            more_data: 0,
            more_frags: 0,
            order: 0,
            pm: 0,
            protected: 0,
            retry: 0,
            stype: 0,
            version: 0,
            bssid,
            source,
            destination,
            seq_ctrl: 0,
            payload: Vec::new(),
        }
        .try_into()
        .unwrap()
    }

    fn test_with_address(
        create_test_ieee80211: fn(MacAddress, MacAddress, MacAddress) -> Ieee80211,
    ) {
        let source = parse_mac_address("01:02:03:00:00:01").unwrap();
        let destination = parse_mac_address("01:02:03:00:00:02").unwrap();
        let bssid = parse_mac_address("00:13:10:85:fe:01").unwrap();
        let ieee80211 = create_test_ieee80211(source, destination, bssid);

        let new_source = parse_mac_address("01:02:03:00:00:03").unwrap();
        let new_destination = parse_mac_address("01:02:03:00:00:04").unwrap();

        let new_ieee80211 = ieee80211.with_address(Some(new_source), Some(new_destination));
        assert!(new_ieee80211.get_source() == new_source);
        assert!(new_ieee80211.get_destination() == new_destination);

        let new_ieee80211 = ieee80211.with_address(Some(new_source), None);
        assert!(new_ieee80211.get_source() == new_source);
        assert!(new_ieee80211.get_destination() == destination);

        let new_ieee80211 = ieee80211.with_address(None, Some(new_destination));
        assert!(new_ieee80211.get_source() == source);
        assert!(new_ieee80211.get_destination() == new_destination);
    }

    #[test]
    fn test_with_address_from_ap() {
        test_with_address(create_test_from_ap_ieee80211);
    }

    #[test]
    fn test_with_address_to_ap() {
        test_with_address(create_test_to_ap_ieee80211);
    }
    #[test]
    fn test_with_address_ibss() {
        test_with_address(create_test_ibss_ieee80211);
    }
}
