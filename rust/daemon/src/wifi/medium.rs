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

use super::packets::mac80211_hwsim::{HwsimAttr, HwsimCmd, HwsimMsg, HwsimMsgHdr};
use super::packets::netlink::{NlAttrHdr, NlMsgHdr};
use crate::devices::chip::ChipIdentifier;
use crate::wifi::frame::Frame;
use crate::wifi::hwsim_attr_set::HwsimAttrSet;
use anyhow::{anyhow, Context};
use log::{debug, info, warn};

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

/// Process commands from the kernel's mac80211_hwsim subsystem.
///
/// This is the processing that will be implemented:
///
/// * The source MacAddress in 802.11 frames is re-mapped to a globally
/// unique MacAddress because resumed Emulator AVDs appear with the
/// same address.
///
/// * 802.11 multicast frames are re-broadcast to connected stations.
///
pub fn process(chip_id: ChipIdentifier, packet: &[u8]) -> anyhow::Result<()> {
    let hwsim_msg = HwsimMsg::parse(packet)?;
    match (hwsim_msg.hwsim_hdr.hwsim_cmd) {
        HwsimCmd::Frame => {
            let frame = Frame::parse(&hwsim_msg)?;
            info!(
                "Frame chip {}, addr {}, flags {}, cookie {:?}, ieee80211 {}",
                chip_id, frame.transmitter, frame.flags, frame.cookie, frame.ieee80211
            );
        }
        HwsimCmd::AddMacAddr => {
            let attr_set = HwsimAttrSet::parse(&hwsim_msg.attributes)?;
            if let (Some(addr), Some(hwaddr)) = (attr_set.transmitter, attr_set.receiver) {
                info!("ADD_MAC_ADDR transmitter {:?} receiver {:?}", hwaddr, addr);
            } else {
                warn!("ADD_MAC_ADDR missing transmitter or receiver");
            }
        }
        HwsimCmd::DelMacAddr => {
            let attr_set = HwsimAttrSet::parse(&hwsim_msg.attributes)?;
            if let (Some(addr), Some(hwaddr)) = (attr_set.transmitter, attr_set.receiver) {
                info!("DEL_MAC_ADDR transmitter {:?} receiver {:?}", hwaddr, addr);
            } else {
                warn!("DEL_MAC_ADDR missing transmitter or receiver");
            }
        }
        _ => {
            info!("Another command found {:?}", hwsim_msg);
        }
    }
    Ok(())
}

// TODO: move code below here into test module usable from CMake

pub fn parse_hwsim_cmd(packet: &[u8]) -> anyhow::Result<HwsimCmdEnum> {
    let hwsim_msg = HwsimMsg::parse(packet)?;
    match (hwsim_msg.hwsim_hdr.hwsim_cmd) {
        HwsimCmd::Frame => {
            let frame = Frame::parse(&hwsim_msg)?;
            Ok(HwsimCmdEnum::Frame(Box::new(frame)))
        }
        _ => Err(anyhow!("Unknown HwsimMsg cmd={:?}", hwsim_msg.hwsim_hdr.hwsim_cmd)),
    }
}

pub fn test_parse_hwsim_cmd() {
    let packet: Vec<u8> = vec![
        188, 0, 0, 0, 34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1, 0, 0, 10, 0, 2, 0, 2, 21, 178, 0,
        0, 0, 0, 0, 98, 0, 3, 0, 64, 0, 0, 0, 255, 255, 255, 255, 255, 255, 74, 129, 38, 251, 211,
        154, 255, 255, 255, 255, 255, 255, 128, 12, 0, 0, 1, 8, 2, 4, 11, 22, 12, 18, 24, 36, 50,
        4, 48, 72, 96, 108, 45, 26, 126, 16, 27, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 255, 22, 35, 1, 120, 200, 26, 64, 0, 0, 191, 206, 0, 0, 0, 0, 0, 0,
        0, 0, 250, 255, 250, 255, 0, 0, 8, 0, 4, 0, 2, 0, 0, 0, 8, 0, 19, 0, 118, 9, 0, 0, 12, 0,
        7, 0, 0, 1, 255, 0, 255, 0, 255, 0, 16, 0, 21, 0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0,
        12, 0, 8, 0, 201, 0, 0, 0, 0, 0, 0, 0,
    ];
    assert!(parse_hwsim_cmd(&packet).is_ok());

    // missing transmitter attribute
    let packet2: Vec<u8> = vec![
        132, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 10, 0, 1, 0, 2, 21, 178, 0,
        0, 0, 0, 0, 76, 0, 3, 0, 8, 2, 0, 0, 2, 21, 178, 0, 0, 0, 0, 19, 16, 133, 254, 1, 82, 85,
        10, 0, 2, 2, 0, 0, 170, 170, 3, 0, 0, 0, 8, 0, 69, 0, 0, 40, 0, 14, 0, 0, 64, 6, 177, 19,
        142, 251, 46, 164, 10, 0, 2, 16, 1, 187, 198, 28, 0, 0, 250, 220, 35, 200, 197, 208, 80,
        16, 255, 255, 57, 216, 0, 0, 8, 0, 5, 0, 1, 0, 0, 0, 8, 0, 6, 0, 206, 255, 255, 255, 8, 0,
        19, 0, 143, 9, 0, 0,
    ];
    assert!(parse_hwsim_cmd(&packet2).is_err());

    // missing cookie attribute
    let packet3: Vec<u8> = vec![
        144, 1, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 10, 0, 1, 0, 2, 21, 178, 0,
        0, 0, 0, 0, 85, 1, 3, 0, 128, 0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 19, 16, 133, 254,
        1, 0, 19, 16, 133, 254, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 232, 3, 1, 4, 0, 11, 65, 110, 100,
        114, 111, 105, 100, 87, 105, 102, 105, 1, 4, 130, 132, 139, 150, 3, 1, 8, 42, 1, 7, 45, 26,
        12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 61, 22, 8,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 4, 0, 0, 0, 2, 128, 0,
        0, 0, 255, 255, 255, 255, 255, 255, 0, 19, 16, 133, 254, 1, 0, 19, 16, 133, 254, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 232, 3, 1, 4, 0, 11, 65, 110, 100, 114, 111, 105, 100, 87, 105,
        102, 105, 1, 4, 130, 132, 139, 150, 3, 1, 8, 42, 1, 7, 45, 26, 12, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 61, 22, 8, 0, 19, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 4, 0, 0, 0, 2, 16, 0, 0, 0, 2, 21, 178, 0, 0, 0,
        0, 19, 16, 133, 254, 1, 0, 19, 16, 133, 254, 1, 0, 0, 1, 4, 0, 0, 1, 192, 1, 4, 130, 132,
        139, 150, 45, 26, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 61, 22, 8, 0, 19, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 4, 0,
        0, 0, 2, 90, 3, 36, 1, 0, 0, 0, 0, 8, 0, 5, 0, 1, 0, 0, 0, 8, 0, 6, 0, 206, 255, 255, 255,
        8, 0, 19, 0, 143, 9, 0, 0,
    ];
    assert!(parse_hwsim_cmd(&packet3).is_err());

    // HwsimkMsg cmd=TxInfoFrame packet
    let packet3: Vec<u8> = vec![
        72, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 10, 0, 2, 0, 2, 21, 178, 0,
        0, 0, 0, 0, 8, 0, 4, 0, 4, 0, 0, 0, 12, 0, 8, 0, 60, 0, 0, 0, 0, 0, 0, 0, 8, 0, 6, 0, 206,
        255, 255, 255, 12, 0, 7, 0, 3, 0, 0, 0, 0, 0, 255, 0,
    ];
    assert!(parse_hwsim_cmd(&packet3).is_err());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netlink_attr() {
        test_parse_hwsim_cmd();
    }
}
