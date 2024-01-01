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

use super::ieee80211::MacAddress;
use super::packets::mac80211_hwsim::HwsimAttrChild::*;
use super::packets::mac80211_hwsim::{HwsimAttr, HwsimMsg, HwsimMsgHdr, TxRate, TxRateFlag};
use super::packets::netlink::{NlAttrHdr, NlMsgHdr};
use anyhow::{anyhow, Context};
use log::{info, warn};

// Decode the hwsim attributes into a set.
//
// Hwsim attributes are used to exchange data between kernel's
// mac80211_hwsim subsystem and this user space process and include:
//
//   HWSIM_ATTR_ADDR_TRANSMITTER,
//   HWSIM_ATTR_ADDR_RECEIVER,
//   HWSIM_ATTR_FRAME,
//   HWSIM_ATTR_FLAGS,
//   HWSIM_ATTR_RX_RATE,
//   HWSIM_ATTR_SIGNAL,
//   HWSIM_ATTR_COOKIE,
//   HWSIM_ATTR_FREQ (optional)
//   HWSIM_ATTR_TX_INFO (new use)
//   HWSIM_ATTR_TX_INFO_FLAGS (new use)

const NLA_ALIGNTO: usize = 4;

fn nla_align(len: usize) -> usize {
    len.wrapping_add(NLA_ALIGNTO - 1) & !(NLA_ALIGNTO - 1)
}

#[derive(Default)]
struct HwsimAttrSetBuilder {
    transmitter: Option<MacAddress>,
    receiver: Option<MacAddress>,
    frame: Option<Vec<u8>>,
    flags: Option<u32>,
    rx_rate_idx: Option<u32>,
    signal: Option<u32>,
    cookie: Option<u64>,
    freq: Option<u32>,
    tx_info: Option<Vec<TxRate>>,
    tx_rate_flags: Option<Vec<TxRateFlag>>,
}

#[derive(Debug)]
pub struct HwsimAttrSet {
    pub transmitter: Option<MacAddress>,
    pub receiver: Option<MacAddress>,
    pub frame: Option<Vec<u8>>,
    pub flags: Option<u32>,
    pub rx_rate_idx: Option<u32>,
    pub signal: Option<u32>,
    pub cookie: Option<u64>,
    pub freq: Option<u32>,
    pub tx_info: Option<Vec<TxRate>>,
    pub tx_rate_flags: Option<Vec<TxRateFlag>>,
}

impl HwsimAttrSetBuilder {
    fn transmitter(&mut self, transmitter: &[u8; 6]) -> &mut Self {
        self.transmitter = Some(MacAddress::from(transmitter));
        self
    }

    fn receiver(&mut self, receiver: &[u8; 6]) -> &mut Self {
        self.receiver = Some(MacAddress::from(receiver));
        self
    }

    fn frame(&mut self, frame: &[u8]) -> &mut Self {
        self.frame = Some(frame.to_vec());
        self
    }

    fn flags(&mut self, flags: u32) -> &mut Self {
        self.flags = Some(flags);
        self
    }

    fn rx_rate(&mut self, rx_rate_idx: u32) -> &mut Self {
        self.rx_rate_idx = Some(rx_rate_idx);
        self
    }

    fn signal(&mut self, signal: u32) -> &mut Self {
        self.signal = Some(signal);
        self
    }

    fn cookie(&mut self, cookie: u64) -> &mut Self {
        self.cookie = Some(cookie);
        self
    }

    fn freq(&mut self, freq: u32) -> &mut Self {
        self.freq = Some(freq);
        self
    }

    fn tx_info(&mut self, tx_info: &[TxRate]) -> &mut Self {
        self.tx_info = Some(tx_info.to_vec());
        self
    }

    fn tx_rate_flags(&mut self, tx_rate_flags: &[TxRateFlag]) -> &mut Self {
        self.tx_rate_flags = Some(tx_rate_flags.to_vec());
        self
    }

    fn build(mut self) -> anyhow::Result<HwsimAttrSet> {
        Ok(HwsimAttrSet {
            transmitter: self.transmitter,
            receiver: self.receiver,
            cookie: self.cookie,
            flags: self.flags,
            rx_rate_idx: self.rx_rate_idx,
            signal: self.signal,
            frame: self.frame,
            freq: self.freq,
            tx_info: self.tx_info,
            tx_rate_flags: self.tx_rate_flags,
        })
    }
}

impl HwsimAttrSet {
    fn builder() -> HwsimAttrSetBuilder {
        HwsimAttrSetBuilder::default()
    }

    // Builds and validates the attributes in the command.
    pub fn parse(attributes: &[u8]) -> anyhow::Result<HwsimAttrSet> {
        let mut index: usize = 0;
        let mut builder = HwsimAttrSet::builder();
        while (index < attributes.len()) {
            // Parse a generic netlink attribute to get the size
            let nla = NlAttrHdr::parse(&attributes[index..index + 4]).unwrap();
            let nla_len = nla.nla_len as usize;
            let hwsim_attr = HwsimAttr::parse(&attributes[index..index + nla_len])?;
            match hwsim_attr.specialize() {
                HwsimAttrAddrTransmitter(child) => builder.transmitter(child.get_address()),
                HwsimAttrAddrReceiver(child) => builder.receiver(child.get_address()),
                HwsimAttrFrame(child) => builder.frame(child.get_data()),
                HwsimAttrFlags(child) => builder.flags(child.get_flags()),
                HwsimAttrRxRate(child) => builder.rx_rate(child.get_rx_rate_idx()),
                HwsimAttrSignal(child) => builder.signal(child.get_signal()),
                HwsimAttrCookie(child) => builder.cookie(child.get_cookie()),
                HwsimAttrFreq(child) => builder.freq(child.get_freq()),
                HwsimAttrTxInfo(child) => builder.tx_info(child.get_tx_rates()),
                HwsimAttrTxInfoFlags(child) => builder.tx_rate_flags(child.get_tx_rate_flags()),
                _ => {
                    return Err(anyhow!(
                        "Invalid attribute message: {:?}",
                        hwsim_attr.get_nla_type() as u32
                    ))
                }
            };
            // Manually step through the attribute bytes aligning as
            // we go because netlink aligns each attribute which isn't
            // a feature of PDL parser.
            index += nla_align(nla_len);
        }
        builder.build()
    }
}
