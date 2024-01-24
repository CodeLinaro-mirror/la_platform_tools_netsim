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

use super::ieee80211::{Ieee80211, Ieee80211Child, Ieee80211FromApBuilder, MacAddress};
use super::packets::mac80211_hwsim::{
    HwsimAttr, HwsimCmd, HwsimMsg, HwsimMsgBuilder, HwsimMsgHdr, NlMsgHdr,
};
use super::packets::netlink::NlAttrHdr;
use crate::devices::chip::ChipIdentifier;
use crate::wifi::frame::Frame;
use crate::wifi::hwsim_attr_set::HwsimAttrSet;
use anyhow::{anyhow, Context};
use log::{debug, info, warn};
use pdl_runtime::Packet;
use std::collections::{HashMap, HashSet};

const NLMSG_MIN_TYPE: u16 = 0x10;
// Default values for mac80211_hwsim.
const RX_RATE: u32 = 0;
const SIGNAL: u32 = 4294967246; // -50

#[derive(Debug)]
pub enum HwsimCmdEnum {
    Unspec,
    Register,
    Frame(Box<Frame>),
    TxInfoFrame,
    NewRadio,
    DelRadio,
    GetRadio,
    AddMacAddr,
    DelMacAddr,
}

struct Station {
    client_id: u32,
    addr: MacAddress, // virtual interface mac address
}

pub struct Medium {
    callback: HwsimCmdCallback,
    stations: HashMap<MacAddress, Station>,
}

type HwsimCmdCallback = fn(u32, &[u8]);

impl Medium {
    pub fn new(callback: HwsimCmdCallback) -> Medium {
        Self { callback, stations: HashMap::new() }
    }
    fn get_station_by_addr(&self, addr: MacAddress) -> Option<&Station> {
        self.stations.get(&addr)
    }
}

impl Station {
    fn new(client_id: u32, addr: MacAddress) -> Self {
        Self { client_id, addr }
    }
}

/// Process commands from the kernel's mac80211_hwsim subsystem.
///
/// This is the processing that will be implemented:
///
/// * The source MacAddress in 802.11 frames is re-mapped to a globally
/// unique MacAddress because resumed Emulator AVDs appear with the
/// same address.
///
/// * 802.11 frames sent between stations
///
/// * 802.11 multicast frames are re-broadcast to connected stations.
///

impl Medium {
    pub fn process(&mut self, client_id: u32, packet: &[u8]) -> bool {
        match self.process_internal(client_id, packet) {
            Ok(b) => b,
            Err(e) => {
                warn!("error processing wifi {e}");
                // continue processing hwsim cmd on error
                false
            }
        }
    }

    fn process_internal(&mut self, client_id: u32, packet: &[u8]) -> anyhow::Result<bool> {
        let hwsim_msg = HwsimMsg::parse(packet)?;
        match (hwsim_msg.get_hwsim_hdr().hwsim_cmd) {
            HwsimCmd::Frame => {
                let frame = Frame::parse(&hwsim_msg)?;
                // Incoming packet must contain transmitter, flag, cookie, and tx_info fields.
                if frame.tx_info.is_none() {
                    return Err(anyhow!("Missing tx_info for incoming packet"));
                }
                let addr = frame.transmitter.context("transmitter")?;
                let flags = frame.flags.context("flags")?;
                let cookie = frame.cookie.context("cookie")?;
                info!(
                    "Frame chip {}, addr {}, flags {}, cookie {}, ieee80211 {}",
                    client_id, addr, flags, cookie, frame.ieee80211
                );
                // Creates Stations on the fly when there is no config file
                let _ = self.stations.entry(addr).or_insert_with(|| Station::new(client_id, addr));
                let sender = self.stations.get(&addr).unwrap();
                self.queue_frame(sender, frame)
            }
            HwsimCmd::AddMacAddr => {
                let attr_set = HwsimAttrSet::parse(hwsim_msg.get_attributes())?;
                if let (Some(addr), Some(hwaddr)) = (attr_set.transmitter, attr_set.receiver) {
                    info!("ADD_MAC_ADDR transmitter {:?} receiver {:?}", hwaddr, addr);
                } else {
                    warn!("ADD_MAC_ADDR missing transmitter or receiver");
                }
                Ok(false)
            }
            HwsimCmd::DelMacAddr => {
                let attr_set = HwsimAttrSet::parse(hwsim_msg.get_attributes())?;
                if let (Some(addr), Some(hwaddr)) = (attr_set.transmitter, attr_set.receiver) {
                    info!("DEL_MAC_ADDR transmitter {:?} receiver {:?}", hwaddr, addr);
                } else {
                    warn!("DEL_MAC_ADDR missing transmitter or receiver");
                }
                Ok(false)
            }
            _ => {
                info!("Another command found {:?}", hwsim_msg);
                Ok(false)
            }
        }
    }

    /// Create tx info frame to station to ack HwsimMsg.
    fn send_tx_info_frame(&self, station: &Station, frame: &Frame) {
        let hwsim_msg_tx_info = build_tx_info(&frame.hwsim_msg).unwrap().to_vec();
        (self.callback)(station.client_id, &hwsim_msg_tx_info);
    }

    /// Create from_ap packet and forward it to destination station.
    fn send_from_ap_packet(&self, station: &Station, dest_station: &Station, frame: &Frame) {
        if let Some(from_ap_packet) = create_from_ap_packet(frame, &dest_station.addr) {
            (self.callback)(dest_station.client_id, &from_ap_packet);
            log_from_ap_packet(&from_ap_packet, station, dest_station);
        }
    }

    /// Create from_ap packet and forward it to other stations.
    fn send_from_ap_packet_to_stations(&self, station: &Station, frame: &Frame) {
        for (mac_address, dest_station) in &self.stations {
            if station.client_id != dest_station.client_id {
                self.send_from_ap_packet(station, dest_station, frame);
            }
        }
    }

    fn queue_frame(&self, station: &Station, frame: Frame) -> anyhow::Result<bool> {
        let destination = frame.ieee80211.get_destination();
        if let Some(dest_station) = self.stations.get(&destination) {
            info!("Frame deliver from {} to {}", station.addr, dest_station.addr);
            self.send_tx_info_frame(station, &frame);
            self.send_from_ap_packet(station, dest_station, &frame);
            Ok(true)
        } else if destination.is_multicast() {
            info!("Frame multicast {}", frame.ieee80211);
            self.send_tx_info_frame(station, &frame);
            self.send_from_ap_packet_to_stations(station, &frame);
            Ok(true)
        } else if destination.is_broadcast() {
            info!("Frame broadcast {}", frame.ieee80211);
            self.send_from_ap_packet_to_stations(station, &frame);
            Ok(false)
        } else {
            // pass to libslirp
            Ok(false)
        }
    }
}

fn log_from_ap_packet(hwsim_msg_from_ap_bytes: &[u8], source: &Station, destination: &Station) {
    let hwsim_msg = HwsimMsg::parse(hwsim_msg_from_ap_bytes).unwrap();
    let frame = Frame::parse(&hwsim_msg).unwrap();
    info!(
        "Sent from_ap packet from client {} to {}. flags {:?}, ieee80211 {}",
        source.client_id, destination.client_id, frame.flags, frame.ieee80211,
    );
}

/// Build TxInfoFrame HwsimMsg from CmdFrame HwsimMsg.
///
/// Reference to ackLocalFrame() in external/qemu/android-qemu2-glue/emulation/VirtioWifiForwarder.cpp
fn build_tx_info(hwsim_msg: &HwsimMsg) -> anyhow::Result<HwsimMsg> {
    let attrs = HwsimAttrSet::parse(hwsim_msg.get_attributes()).context("HwsimAttrSet").unwrap();

    let hwsim_hdr = hwsim_msg.get_hwsim_hdr();
    let nl_hdr = hwsim_msg.get_nl_hdr();
    let mut new_attr_builder = HwsimAttrSet::builder();
    const HWSIM_TX_STAT_ACK: u32 = 1 << 2;

    new_attr_builder
        .transmitter(&attrs.transmitter.context("transmitter")?.into())
        .flags(attrs.flags.context("flags")? | HWSIM_TX_STAT_ACK)
        .cookie(attrs.cookie.context("cookie")?)
        .signal(attrs.signal.unwrap_or(SIGNAL))
        .tx_info(attrs.tx_info.context("tx_info")?.as_slice());

    let new_attr = new_attr_builder.build().unwrap();
    let nlmsg_len =
        nl_hdr.nlmsg_len + new_attr.attributes.len() as u32 - attrs.attributes.len() as u32;
    let new_hwsim_msg = HwsimMsgBuilder {
        attributes: new_attr.attributes,
        hwsim_hdr: HwsimMsgHdr {
            hwsim_cmd: HwsimCmd::TxInfoFrame,
            hwsim_version: 0,
            reserved: hwsim_hdr.reserved,
        },
        nl_hdr: NlMsgHdr {
            nlmsg_len,
            nlmsg_type: NLMSG_MIN_TYPE,
            nlmsg_flags: nl_hdr.nlmsg_flags,
            nlmsg_seq: 0,
            nlmsg_pid: 0,
        },
    }
    .build();
    Ok(new_hwsim_msg)
}

fn create_from_ap_attributes(
    attributes: &[u8],
    receiver: &MacAddress,
    attrs: &HwsimAttrSet,
) -> anyhow::Result<Vec<u8>> {
    let ieee80211_frame = Ieee80211::parse(&attrs.frame.clone().context("frame")?)?;
    let new_frame = ieee80211_frame.into_from_ap()?.to_vec();

    let mut builder = HwsimAttrSet::builder();

    // Attributes required by mac80211_hwsim.
    builder.receiver(&receiver.to_vec());
    builder.frame(&new_frame);
    // NOTE: Incoming mdns packets don't have rx_rate and signal.
    builder.rx_rate(attrs.rx_rate_idx.unwrap_or(RX_RATE));
    builder.signal(attrs.signal.unwrap_or(SIGNAL));

    attrs.flags.map(|v| builder.flags(v));
    attrs.freq.map(|v| builder.freq(v));
    attrs.tx_info.as_ref().map(|v| builder.tx_info(v));
    attrs.tx_info_flags.as_ref().map(|v| builder.tx_info_flags(v));

    Ok(builder.build()?.attributes)
}

fn create_from_ap_packet(frame: &Frame, receiver: &MacAddress) -> Option<Vec<u8>> {
    let hwsim_msg = &frame.hwsim_msg;
    assert_eq!(hwsim_msg.get_hwsim_hdr().hwsim_cmd, HwsimCmd::Frame);
    let attributes_result =
        create_from_ap_attributes(hwsim_msg.get_attributes(), receiver, &frame.attrs);
    let attributes = match attributes_result {
        Ok(attributes) => attributes,
        Err(e) => {
            warn!("Failed to create from_ap attributes. E: {}", e);
            return None;
        }
    };

    let nlmsg_len = hwsim_msg.get_nl_hdr().nlmsg_len + attributes.len() as u32
        - hwsim_msg.get_attributes().len() as u32;
    let new_hwsim_msg = HwsimMsgBuilder {
        nl_hdr: NlMsgHdr {
            nlmsg_len,
            nlmsg_type: NLMSG_MIN_TYPE,
            nlmsg_flags: hwsim_msg.get_nl_hdr().nlmsg_flags,
            nlmsg_seq: 0,
            nlmsg_pid: 0,
        },
        hwsim_hdr: hwsim_msg.get_hwsim_hdr().clone(),
        attributes,
    }
    .build();
    Some(new_hwsim_msg.to_vec())
}

// It's used by radiotap.rs for packet capture.
pub fn parse_hwsim_cmd(packet: &[u8]) -> anyhow::Result<HwsimCmdEnum> {
    let hwsim_msg = HwsimMsg::parse(packet)?;
    match (hwsim_msg.get_hwsim_hdr().hwsim_cmd) {
        HwsimCmd::Frame => {
            let frame = Frame::parse(&hwsim_msg)?;
            Ok(HwsimCmdEnum::Frame(Box::new(frame)))
        }
        HwsimCmd::TxInfoFrame => Ok(HwsimCmdEnum::TxInfoFrame),
        _ => Err(anyhow!("Unknown HwsimMsg cmd={:?}", hwsim_msg.get_hwsim_hdr().hwsim_cmd)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netlink_attr() {
        let packet: Vec<u8> = include!("test_packets/hwsim_cmd_frame.csv");
        assert!(parse_hwsim_cmd(&packet).is_ok());

        let tx_info_packet: Vec<u8> = include!("test_packets/hwsim_cmd_tx_info.csv");
        assert!(parse_hwsim_cmd(&tx_info_packet).is_ok());
    }

    #[test]
    fn test_netlink_attr_response_packet() {
        // Response packet may not contain transmitter, flags, tx_info, or cookie fields.
        let response_packet: Vec<u8> =
            include!("test_packets/hwsim_cmd_frame_response_no_transmitter_flags_tx_info.csv");
        assert!(parse_hwsim_cmd(&response_packet).is_ok());

        let response_packet2: Vec<u8> =
            include!("test_packets/hwsim_cmd_frame_response_no_cookie.csv");
        assert!(parse_hwsim_cmd(&response_packet2).is_ok());
    }

    #[test]
    fn test_is_mdns_packet() {
        let packet: Vec<u8> = include!("test_packets/hwsim_cmd_frame_mdns.csv");
        let hwsim_msg = HwsimMsg::parse(&packet).unwrap();
        let mdns_frame = Frame::parse(&hwsim_msg).unwrap();
        assert!(mdns_frame.ieee80211.get_destination().is_multicast());

        let packet: Vec<u8> = include!("test_packets/hwsim_cmd_frame.csv");
        let hwsim_msg = HwsimMsg::parse(&packet).unwrap();
        let non_mdns_frame = Frame::parse(&hwsim_msg).unwrap();
        assert!(!non_mdns_frame.ieee80211.get_destination().is_multicast());
    }

    #[test]
    fn test_build_tx_info_reconstruct() {
        let packet: Vec<u8> = include!("test_packets/hwsim_cmd_tx_info.csv");
        let hwsim_msg = HwsimMsg::parse(&packet).unwrap();
        assert_eq!(hwsim_msg.get_hwsim_hdr().hwsim_cmd, HwsimCmd::TxInfoFrame);

        let new_hwsim_msg = build_tx_info(&hwsim_msg).unwrap();
        assert_eq!(hwsim_msg, new_hwsim_msg);
    }

    #[test]
    fn test_build_tx_info() {
        let packet: Vec<u8> = include!("test_packets/hwsim_cmd_frame.csv");
        let hwsim_msg = HwsimMsg::parse(&packet).unwrap();
        let hwsim_msg_tx_info = build_tx_info(&hwsim_msg).unwrap();
        assert_eq!(hwsim_msg_tx_info.get_hwsim_hdr().hwsim_cmd, HwsimCmd::TxInfoFrame);
    }

    fn build_tx_info_and_compare(frame_bytes: &[u8], tx_info_expected_bytes: &[u8]) {
        let frame = HwsimMsg::parse(frame_bytes).unwrap();
        let tx_info = build_tx_info(&frame).unwrap();

        let tx_info_expected = HwsimMsg::parse(tx_info_expected_bytes).unwrap();

        assert_eq!(tx_info.get_hwsim_hdr(), tx_info_expected.get_hwsim_hdr());
        assert_eq!(tx_info.get_nl_hdr(), tx_info_expected.get_nl_hdr());

        let attrs = HwsimAttrSet::parse(tx_info.get_attributes()).context("HwsimAttrSet").unwrap();
        let attrs_expected =
            HwsimAttrSet::parse(tx_info_expected.get_attributes()).context("HwsimAttrSet").unwrap();

        // NOTE: TX info is different and the counts are all zeros in the TX info packet generated by WifiService.
        // TODO: Confirm if the behavior is intended in WifiService.
        assert_eq!(attrs.transmitter, attrs_expected.transmitter);
        assert_eq!(attrs.flags, attrs_expected.flags);
        assert_eq!(attrs.cookie, attrs_expected.cookie);
        assert_eq!(attrs.signal, attrs_expected.signal);
    }

    #[test]
    fn test_build_tx_info_and_compare() {
        let frame_bytes: Vec<u8> = include!("test_packets/hwsim_cmd_frame_request.csv");
        let tx_info_expected_bytes: Vec<u8> =
            include!("test_packets/hwsim_cmd_tx_info_response.csv");
        build_tx_info_and_compare(&frame_bytes, &tx_info_expected_bytes);
    }

    #[test]
    fn test_build_tx_info_and_compare_mdns() {
        let frame_bytes: Vec<u8> = include!("test_packets/hwsim_cmd_frame_request_mdns.csv");
        let tx_info_expected_bytes: Vec<u8> =
            include!("test_packets/hwsim_cmd_tx_info_response_mdns.csv");
        build_tx_info_and_compare(&frame_bytes, &tx_info_expected_bytes);
    }
}
