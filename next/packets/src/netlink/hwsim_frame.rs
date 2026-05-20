// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! High-level Hwsim Frame wrapper.

use crate::{
    ethernet::MacAddr,
    ieee80211::Ieee80211,
    netlink::{
        TxRate,
        hwsim_attr_set::{HwsimAttrSet, HwsimError},
        mac80211_hwsim::{HwsimCmd, HwsimMsg},
    },
};

/// Parser for the hwsim Frame command (HWSIM_CMD_FRAME).
///
/// The Frame command is sent by the kernel's mac80211_hwsim subsystem
/// and contains the IEEE 802.11 frame along with hwsim attributes.
#[derive(Debug, Clone)]
pub struct HwsimFrame {
    /// Transmitter MAC address.
    pub transmitter: Option<MacAddr>,
    /// Flags.
    pub flags: Option<u32>,
    /// Transmit info (rates).
    pub tx_info: Option<Vec<TxRate>>,
    /// Cookie.
    pub cookie: Option<u64>,
    /// Signal strength.
    pub signal: Option<u32>,
    /// Frequency.
    pub freq: Option<u32>,
    /// Raw frame data.
    pub data: Vec<u8>,
    /// Parsed IEEE 802.11 frame.
    pub ieee80211: Ieee80211,
    /// Original Hwsim message.
    pub hwsim_msg: HwsimMsg,
    /// Parsed Hwsim attributes.
    pub attrs: HwsimAttrSet,
}

impl HwsimFrame {
    /// Builds and validates the Frame from the attributes in the packet.
    pub fn parse(msg: &HwsimMsg) -> Result<HwsimFrame, HwsimError> {
        // Only expected to be called with HwsimCmd::Frame
        if msg.hwsim_hdr.hwsim_cmd != HwsimCmd::Frame {
            return Err(HwsimError::Frame(format!(
                "Invalid hwsim_cmd: {:?}",
                msg.hwsim_hdr.hwsim_cmd
            )));
        }
        let attrs = HwsimAttrSet::parse(&msg.attributes)?;
        let frame =
            attrs.frame.clone().ok_or(HwsimError::Frame("Missing frame attribute".to_string()))?;
        let ieee80211 = Ieee80211::decode_full(&frame)
            .map_err(|e| HwsimError::Frame(format!("Failed to decode Ieee80211: {}", e)))?;

        Ok(HwsimFrame {
            transmitter: attrs.transmitter,
            flags: attrs.flags,
            tx_info: attrs.tx_info.clone(),
            cookie: attrs.cookie,
            signal: attrs.signal,
            freq: attrs.freq,
            data: frame,
            ieee80211,
            hwsim_msg: msg.clone(),
            attrs,
        })
    }
}
