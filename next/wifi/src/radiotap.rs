#![allow(dead_code)]
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

/// Produce PCAP Radiotap buffers from Hwsim Frames
///
/// This module produces the Radiotap buffers used by PCAP and PcapNG
/// for logging 802.11 frames.
///
/// See https://www.radiotap.org/
use crate::error::{WifiError, WifiResult};
use log::info;
use netsim_packets::netlink::hwsim_frame::HwsimFrame;
use netsim_packets::netlink::{HwsimCmd, HwsimMsg};
use netsim_packets::pcap::radiotap::create_radiotap_packet;

#[allow(dead_code)]
#[derive(Debug)]
enum HwsimCmdEnum {
    Unspec,
    Register,
    Frame(Box<HwsimFrame>),
    TxInfoFrame,
    NewRadio,
    DelRadio,
    GetRadio,
    AddMacAddr,
    DelMacAddr,
}

fn parse_hwsim_cmd(packet: &[u8]) -> WifiResult<HwsimCmdEnum> {
    let hwsim_msg = HwsimMsg::decode_full(packet).map_err(WifiError::Frame)?;
    match hwsim_msg.hwsim_hdr.hwsim_cmd {
        HwsimCmd::Frame => {
            let frame =
                HwsimFrame::parse(&hwsim_msg).map_err(|e| WifiError::Frame(e.to_string()))?;
            Ok(HwsimCmdEnum::Frame(Box::new(frame)))
        }
        HwsimCmd::TxInfoFrame => Ok(HwsimCmdEnum::TxInfoFrame),
        _ => Err(WifiError::Other(format!(
            "Unknown HwsimMsg cmd={:?}",
            hwsim_msg.hwsim_hdr.hwsim_cmd
        ))),
    }
}

pub fn into_pcap(packet: &[u8]) -> Option<Vec<u8>> {
    match parse_hwsim_cmd(packet) {
        Ok(HwsimCmdEnum::Frame(frame)) => Some(create_radiotap_packet(&frame)),
        Ok(_) => None,
        Err(e) => {
            info!("Failed to convert packet to pcap format. Err: {}. Packet: {:?}", e, &packet);
            None
        }
    }
}

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

    let response_packet2: Vec<u8> = include!("test_packets/hwsim_cmd_frame_response_no_cookie.csv");
    assert!(parse_hwsim_cmd(&response_packet2).is_ok());
}
