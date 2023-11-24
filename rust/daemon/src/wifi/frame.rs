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

use super::packets::mac80211_hwsim::HwsimAttrChild::*;
use super::packets::mac80211_hwsim::{HwsimAttr, HwsimMsg, HwsimMsgHdr, TxRate, TxRateFlag};
use super::packets::netlink::{NlAttrHdr, NlMsgHdr};
use crate::wifi::packets::ieee80211::Ieee80211;
use anyhow::{anyhow, Context};
use log::{info, warn};
use std::mem;

// Decode the hwsim Frame.
//
// HWSIM_CMD_FRAME is used to send/receive a broadcasted frame from/to
// kernel/user space, uses these attributes:
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
struct FrameBuilder {
    transmitter: Option<[u8; 6]>,
    receiver: Option<[u8; 6]>,
    data: Option<Vec<u8>>,
    flags: Option<u32>,
    rx_rate_idx: Option<u32>,
    signal: Option<u32>,
    cookie: Option<u64>,
    freq: Option<u32>,
    tx_rates: Option<Vec<TxRate>>,
    tx_rate_flags: Option<Vec<TxRateFlag>>,
}

#[derive(Debug)]
pub struct Frame {
    transmitter: [u8; 6],
    receiver: Option<[u8; 6]>,
    pub data: Vec<u8>,
    pub ieee80211_hdr: Option<Ieee80211>,
    pub flags: u32,
    rx_rate_idx: Option<u32>,
    pub signal: Option<u32>,
    cookie: u64,
    pub freq: Option<u32>,
    tx_rates: Option<Vec<TxRate>>,
    tx_rate_flags: Option<Vec<TxRateFlag>>,
}

fn anymsg(attr: &str) -> anyhow::Error {
    anyhow!("hwsim Frame missing {} attribute", attr)
}

impl FrameBuilder {
    fn transmitter(&mut self, transmitter: &[u8; 6]) -> &mut Self {
        self.transmitter = Some(*transmitter);
        self
    }

    fn receiver(&mut self, receiver: &[u8; 6]) -> &mut Self {
        self.receiver = Some(*receiver);
        self
    }

    fn frame(&mut self, data: &[u8]) -> &mut Self {
        self.data = Some(data.to_vec());
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

    fn tx_rates(&mut self, tx_rates: &[TxRate]) -> &mut Self {
        self.tx_rates = Some(tx_rates.to_vec());
        self
    }

    fn tx_rate_flags(&mut self, tx_rate_flags: &[TxRateFlag]) -> &mut Self {
        self.tx_rate_flags = Some(tx_rate_flags.to_vec());
        self
    }

    fn build(mut self) -> anyhow::Result<Frame> {
        let data = self.data.ok_or(anymsg("frame"))?;
        let ieee80211_hdr = Ieee80211::parse(&data).ok();
        Ok(Frame {
            transmitter: self.transmitter.ok_or(anymsg("transmitter"))?,
            receiver: self.receiver,
            cookie: self.cookie.ok_or(anymsg("cookie"))?,
            flags: self.flags.ok_or(anymsg("flags"))?,
            rx_rate_idx: self.rx_rate_idx,
            signal: self.signal,
            data,
            ieee80211_hdr,
            freq: self.freq,
            tx_rates: self.tx_rates,
            tx_rate_flags: self.tx_rate_flags,
        })
    }
}

impl Frame {
    fn builder() -> FrameBuilder {
        FrameBuilder::default()
    }

    // Builds and validates the Frame from the attributes in the
    // packet. Called when a hwsim packet with HwsimCmd::Frame is
    // found.
    pub fn new(attributes: &[u8]) -> anyhow::Result<Frame> {
        let mut index: usize = 0;
        let mut builder = Frame::builder();
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
                HwsimAttrTxInfo(child) => builder.tx_rates(child.get_tx_rates()),
                HwsimAttrTxInfoFlags(child) => builder.tx_rate_flags(child.get_tx_rate_flags()),
                _ => {
                    return Err(anyhow!(
                        "Invalid attribute in frame: {:?}",
                        hwsim_attr.get_nla_type() as u32
                    ))
                }
            };
            index += nla_align(nla_len);
        }
        builder.build()
    }
}
