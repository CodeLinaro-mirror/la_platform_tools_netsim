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

use super::packets::ieee80211::MacAddress;
use super::packets::mac80211_hwsim::{HwsimCmd, HwsimMsg, HwsimMsgBuilder, HwsimMsgHdr, NlMsgHdr};
use crate::wifi::frame::Frame;
use crate::wifi::hwsim_attr_set::HwsimAttrSet;
use anyhow::{anyhow, Context};
use bytes::Bytes;
use log::{debug, info, warn};
use pdl_runtime::Packet;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
const NLMSG_MIN_TYPE: u16 = 0x10;
// Default values for mac80211_hwsim.
const RX_RATE: u32 = 0;
const SIGNAL: u32 = 4294967246; // -50

#[allow(dead_code)]
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

#[derive(Clone)]
struct Station {
    client_id: u32,
    // Ieee80211 source address
    addr: MacAddress,
    // Hwsim virtual address from HWSIM_ATTR_ADDR_TRANSMITTER
    // Used to create the HwsimMsg to stations.
    hwsim_addr: MacAddress,
}

impl Station {
    fn new(client_id: u32, addr: MacAddress, hwsim_addr: MacAddress) -> Self {
        Self { client_id, addr, hwsim_addr }
    }
}

#[derive(Clone)]
pub struct Client {
    pub enabled: Arc<AtomicBool>,
    pub tx_count: Arc<AtomicU32>,
    pub rx_count: Arc<AtomicU32>,
}

impl Client {
    fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            tx_count: Arc::new(AtomicU32::new(0)),
            rx_count: Arc::new(AtomicU32::new(0)),
        }
    }
}

pub struct Medium {
    callback: HwsimCmdCallback,
    // Ieee80211 source address
    stations: RwLock<HashMap<MacAddress, Arc<Station>>>,
    clients: RwLock<HashMap<u32, Client>>,
    // BSSID. MAC address of the access point in WiFi Service.
    hostapd_bssid: MacAddress,
    // Simulate the re-transmission of frames sent to hostapd
    ap_simulation: bool,
}

type HwsimCmdCallback = fn(u32, &Bytes);
impl Medium {
    pub fn new(callback: HwsimCmdCallback) -> Medium {
        // Defined in external/qemu/android-qemu2-glue/emulation/WifiService.cpp
        // TODO: Use hostapd_bssid to initialize hostapd.
        let bssid_bytes: [u8; 6] = [0x00, 0x13, 0x10, 0x85, 0xfe, 0x01];
        Self {
            callback,
            stations: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
            hostapd_bssid: MacAddress::from(&bssid_bytes),
            ap_simulation: true,
        }
    }

    pub fn add(&self, client_id: u32) {
        let _ = self.clients.write().unwrap().entry(client_id).or_insert_with(|| {
            info!("Insert client {}", client_id);
            Client::new()
        });
    }

    pub fn remove(&self, client_id: u32) {
        self.stations.write().unwrap().retain(|_, s| s.client_id != client_id);
        self.clients.write().unwrap().remove(&client_id);
    }

    pub fn reset(&self, client_id: u32) {
        if let Some(client) = self.clients.read().unwrap().get(&client_id) {
            client.enabled.store(true, Ordering::Relaxed);
            client.tx_count.store(0, Ordering::Relaxed);
            client.rx_count.store(0, Ordering::Relaxed);
        }
    }

    pub fn get(&self, client_id: u32) -> Option<Client> {
        self.clients.read().unwrap().get(&client_id).map(|c| c.to_owned())
    }

    fn contains_client(&self, client_id: u32) -> bool {
        self.clients.read().unwrap().contains_key(&client_id)
    }

    fn stations(&self) -> impl Iterator<Item = Arc<Station>> {
        self.stations.read().unwrap().clone().into_values()
    }

    fn contains_station(&self, addr: &MacAddress) -> bool {
        self.stations.read().unwrap().contains_key(addr)
    }

    fn get_station(&self, addr: &MacAddress) -> anyhow::Result<Arc<Station>> {
        self.stations.read().unwrap().get(addr).context("get station").cloned()
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
    pub fn process(&self, client_id: u32, packet: &Bytes) -> bool {
        self.process_internal(client_id, packet).unwrap_or_else(move |e| {
            // TODO: add this error to the netsim_session_stats
            warn!("error processing wifi {e}");
            false
        })
    }

    fn process_internal(&self, client_id: u32, packet: &Bytes) -> anyhow::Result<bool> {
        let hwsim_msg = HwsimMsg::parse(packet)?;

        // The virtio handler only accepts HWSIM_CMD_FRAME, HWSIM_CMD_TX_INFO_FRAME and HWSIM_CMD_REPORT_PMSR
        // in https://source.corp.google.com/h/kernel/pub/scm/linux/kernel/git/torvalds/linux/+/master:drivers/net/wireless/virtual/mac80211_hwsim.c
        match hwsim_msg.get_hwsim_hdr().hwsim_cmd {
            HwsimCmd::Frame => {
                let frame = Frame::parse(&hwsim_msg)?;
                // Incoming frame must contain transmitter, flag, cookie, and tx_info fields.
                if frame.transmitter.is_none()
                    || frame.flags.is_none()
                    || frame.cookie.is_none()
                    || frame.tx_info.is_none()
                {
                    return Err(anyhow!("Missing Hwsim attributes for incoming packet"));
                }
                // Use as receiver for outgoing HwsimMsg.
                let hwsim_addr = frame.transmitter.context("transmitter")?;
                let flags = frame.flags.context("flags")?;
                let cookie = frame.cookie.context("cookie")?;
                debug!(
                    "Frame chip {}, transmitter {}, flags {}, cookie {}, ieee80211 {}",
                    client_id, hwsim_addr, flags, cookie, frame.ieee80211
                );
                let src_addr = frame.ieee80211.get_source();
                // Creates Stations on the fly when there is no config file.
                // WiFi Direct will use a randomized mac address for probing
                // new networks. This block associates the new mac with the station.
                let source = self
                    .stations
                    .write()
                    .unwrap()
                    .entry(src_addr)
                    .or_insert_with(|| {
                        info!(
                            "Insert station with client id {}, hwsimaddr: {}, \
                        Ieee80211 addr: {}",
                            client_id, hwsim_addr, src_addr
                        );
                        Arc::new(Station::new(client_id, src_addr, hwsim_addr))
                    })
                    .clone();
                if !self.contains_client(client_id) {
                    warn!("Client {} is missing", client_id);
                    self.add(client_id);
                }
                self.queue_frame(frame, &source)
            }
            _ => {
                info!("Another command found {:?}", hwsim_msg);
                Ok(false)
            }
        }
    }

    /// Handle Wi-Fi MwsimMsg from libslirp and hostapd.
    /// Send it to clients.
    pub fn process_response(&self, packet: &Bytes) {
        if let Err(e) = self.send_response(packet) {
            warn!("{}", e);
        }
    }

    /// Determine the client id based on Ieee80211 destination and send to client.
    fn send_response(&self, packet: &Bytes) -> anyhow::Result<()> {
        // When Wi-Fi P2P is disabled, send all packets from WifiService to all clients.
        if crate::config::get_disable_wifi_p2p() {
            for client_id in self.clients.read().unwrap().keys() {
                (self.callback)(*client_id, packet);
            }
            return Ok(());
        }
        let hwsim_msg = HwsimMsg::parse(packet)?;
        let hwsim_cmd = hwsim_msg.get_hwsim_hdr().hwsim_cmd;
        match hwsim_cmd {
            HwsimCmd::Frame => self.send_frame_response(packet, &hwsim_msg)?,
            // TODO: Handle sending TxInfo frame for WifiService so we don't have to
            // send duplicate HwsimMsg for all clients with the same Hwsim addr.
            HwsimCmd::TxInfoFrame => self.send_tx_info_response(packet, &hwsim_msg)?,
            _ => return Err(anyhow!("Invalid HwsimMsg cmd={:?}", hwsim_cmd)),
        };
        Ok(())
    }

    fn send_frame_response(&self, packet: &Bytes, hwsim_msg: &HwsimMsg) -> anyhow::Result<()> {
        let frame = Frame::parse(hwsim_msg)?;
        let dest_addr = frame.ieee80211.get_destination();
        if let Ok(destination) = self.get_station(&dest_addr) {
            self.send_from_ds_frame(packet, &frame, &destination)?;
        } else if dest_addr.is_multicast() {
            for destination in self.stations() {
                self.send_from_ds_frame(packet, &frame, &destination)?;
            }
        } else {
            warn!("Send frame response to unknown destination: {}", dest_addr);
        }
        Ok(())
    }

    /// Send frame from DS to STA.
    fn send_from_ds_frame(
        &self,
        packet: &Bytes,
        frame: &Frame,
        destination: &Station,
    ) -> anyhow::Result<()> {
        if frame.attrs.receiver.context("receiver")? == destination.hwsim_addr {
            (self.callback)(destination.client_id, packet);
        } else {
            // Broadcast: replace HwsimMsg destination but keep other attributes
            let hwsim_msg = self
                .create_hwsim_msg(frame, &destination.hwsim_addr)
                .context("Create HwsimMsg from WifiService")?;
            (self.callback)(destination.client_id, &hwsim_msg.encode_to_vec()?.into());
        }
        self.incr_rx(destination.client_id)?;
        Ok(())
    }

    fn send_tx_info_response(&self, packet: &Bytes, hwsim_msg: &HwsimMsg) -> anyhow::Result<()> {
        let attrs = HwsimAttrSet::parse(hwsim_msg.get_attributes()).context("HwsimAttrSet")?;
        let hwsim_addr = attrs.transmitter.context("missing transmitter")?;
        let client_ids = self
            .stations()
            .filter(|v| v.hwsim_addr == hwsim_addr)
            .map(|v| v.client_id)
            .collect::<HashSet<_>>();
        if client_ids.len() > 1 {
            warn!("multiple clients found for TxInfo frame");
        }
        for client_id in client_ids {
            if self.enabled(client_id)? {
                (self.callback)(client_id, packet);
            }
        }
        Ok(())
    }

    pub fn set_enabled(&self, client_id: u32, enabled: bool) {
        if let Some(client) = self.clients.read().unwrap().get(&client_id) {
            client.enabled.store(enabled, Ordering::Relaxed);
        }
    }

    fn enabled(&self, client_id: u32) -> anyhow::Result<bool> {
        Ok(self
            .clients
            .read()
            .unwrap()
            .get(&client_id)
            .context(format!("client {client_id} is missing"))?
            .enabled
            .load(Ordering::Relaxed))
    }

    /// Create tx info frame to station to ack HwsimMsg.
    fn send_tx_info_frame(&self, frame: &Frame) -> anyhow::Result<()> {
        let client_id = self.get_station(&frame.ieee80211.get_source())?.client_id;
        let hwsim_msg_tx_info = build_tx_info(&frame.hwsim_msg).unwrap().encode_to_vec()?;
        (self.callback)(client_id, &hwsim_msg_tx_info.into());
        Ok(())
    }

    fn incr_tx(&self, client_id: u32) -> anyhow::Result<()> {
        self.clients
            .read()
            .unwrap()
            .get(&client_id)
            .context("incr_tx")?
            .tx_count
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn incr_rx(&self, client_id: u32) -> anyhow::Result<()> {
        self.clients
            .read()
            .unwrap()
            .get(&client_id)
            .context("incr_rx")?
            .rx_count
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    // Send an 802.11 frame from a station to a station after wrapping in HwsimMsg.
    // The HwsimMsg is processed by mac80211_hwsim framework in the guest OS.
    //
    // Simulates transmission through hostapd.
    fn send_from_sta_frame(
        &self,
        frame: &Frame,
        source: &Station,
        destination: &Station,
    ) -> anyhow::Result<()> {
        if self.enabled(source.client_id)? && self.enabled(destination.client_id)? {
            if let Some(packet) = self.create_hwsim_msg(frame, &destination.hwsim_addr) {
                self.incr_tx(source.client_id)?;
                self.incr_rx(destination.client_id)?;
                (self.callback)(destination.client_id, &packet.encode_to_vec()?.into());
                log_hwsim_msg(frame, source.client_id, destination.client_id);
            }
        }
        Ok(())
    }

    // Broadcast an 802.11 frame to all stations.
    /// TODO: Compare with the implementations in mac80211_hwsim.c and wmediumd.c.
    fn broadcast_from_sta_frame(&self, frame: &Frame, source: &Station) -> anyhow::Result<()> {
        for destination in self.stations() {
            if source.addr != destination.addr {
                self.send_from_sta_frame(frame, source, &destination)?;
            }
        }
        Ok(())
    }

    fn queue_frame(&self, frame: Frame, source: &Station) -> anyhow::Result<bool> {
        let dest_addr = frame.ieee80211.get_destination();
        if self.contains_station(&dest_addr) {
            debug!("Frame deliver from {} to {}", source.addr, dest_addr);
            self.send_tx_info_frame(&frame)?;
            let destination = self.get_station(&dest_addr)?;
            self.send_from_sta_frame(&frame, source, &destination)?;
            Ok(true)
        } else if dest_addr.is_multicast() {
            debug!("Frame multicast {}", frame.ieee80211);
            self.send_tx_info_frame(&frame)?;
            self.broadcast_from_sta_frame(&frame, source)?;
            // Forward multicast packets to WifiService:
            // 1. Stations send probe Request management frame scan network actively,
            //    so hostapd will send Probe Response for AndroidWiFi.
            // 2. DNS packets.
            // TODO: Only pass necessary packets to hostapd and libslirp. (e.g.: Don't forward mDNS packet.)
            self.incr_tx(source.client_id)?;
            Ok(false)
        } else {
            // pass to libslirp
            self.incr_tx(source.client_id)?;
            Ok(false)
        }
    }

    // Simulate transmission through hostapd by rewriting frames with 802.11 ToDS
    // and hostapd_bssid to frames with FromDS set.
    fn create_hwsim_attr(
        &self,
        frame: &Frame,
        dest_hwsim_addr: &MacAddress,
    ) -> anyhow::Result<Vec<u8>> {
        let attrs = &frame.attrs;
        let frame = match self.ap_simulation
            && frame.ieee80211.is_to_ap()
            && frame.ieee80211.get_bssid() == Some(self.hostapd_bssid)
        {
            true => frame.ieee80211.into_from_ap()?.encode_to_vec()?,
            false => attrs.frame.clone().unwrap(),
        };

        let mut builder = HwsimAttrSet::builder();

        // Attributes required by mac80211_hwsim.
        builder.receiver(&dest_hwsim_addr.to_vec());
        builder.frame(&frame);
        // Incoming HwsimMsg don't have rx_rate and signal.
        builder.rx_rate(attrs.rx_rate_idx.unwrap_or(RX_RATE));
        builder.signal(attrs.signal.unwrap_or(SIGNAL));

        attrs.flags.map(|v| builder.flags(v));
        attrs.freq.map(|v| builder.freq(v));
        attrs.tx_info.as_ref().map(|v| builder.tx_info(v));
        attrs.tx_info_flags.as_ref().map(|v| builder.tx_info_flags(v));

        Ok(builder.build()?.attributes)
    }

    // Simulates transmission through hostapd.
    fn create_hwsim_msg(&self, frame: &Frame, dest_hwsim_addr: &MacAddress) -> Option<HwsimMsg> {
        let hwsim_msg = &frame.hwsim_msg;
        assert_eq!(hwsim_msg.get_hwsim_hdr().hwsim_cmd, HwsimCmd::Frame);
        let attributes_result = self.create_hwsim_attr(frame, dest_hwsim_addr);
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
        Some(new_hwsim_msg)
    }
}

fn log_hwsim_msg(frame: &Frame, client_id: u32, dest_client_id: u32) {
    debug!(
        "Sent hwsim_msg from client {} to {}. flags {:?}, ieee80211 {}",
        client_id, dest_client_id, frame.flags, frame.ieee80211,
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

// It's used by radiotap.rs for packet capture.
pub fn parse_hwsim_cmd(packet: &[u8]) -> anyhow::Result<HwsimCmdEnum> {
    let hwsim_msg = HwsimMsg::parse(packet)?;
    match hwsim_msg.get_hwsim_hdr().hwsim_cmd {
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
    use crate::wifi::packets::ieee80211::parse_mac_address;
    #[test]
    fn test_remove() {
        let hostapd_bssid: MacAddress = parse_mac_address("00:13:10:85:fe:01").unwrap();

        let test_client_id: u32 = 1234;
        let other_client_id: u32 = 5678;
        let addr: MacAddress = parse_mac_address("00:0b:85:71:20:00").unwrap();
        let other_addr: MacAddress = parse_mac_address("00:0b:85:71:20:01").unwrap();
        let hwsim_addr: MacAddress = parse_mac_address("00:0b:85:71:20:ce").unwrap();
        let other_hwsim_addr: MacAddress = parse_mac_address("00:0b:85:71:20:cf").unwrap();

        // Create a test Medium object
        let callback: HwsimCmdCallback = |_, _| {};
        let medium = Medium {
            callback,
            stations: RwLock::new(HashMap::from([
                (addr, Arc::new(Station { client_id: test_client_id, addr, hwsim_addr })),
                (
                    other_addr,
                    Arc::new(Station {
                        client_id: other_client_id,
                        addr: other_addr,
                        hwsim_addr: other_hwsim_addr,
                    }),
                ),
            ])),
            clients: RwLock::new(HashMap::from([
                (test_client_id, Client::new()),
                (other_client_id, Client::new()),
            ])),
            hostapd_bssid,
            ap_simulation: true,
        };

        medium.remove(test_client_id);

        assert!(!medium.contains_station(&addr));
        assert!(medium.contains_station(&other_addr));
        assert!(!medium.contains_client(test_client_id));
        assert!(medium.contains_client(other_client_id));
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

        let response_packet2: Vec<u8> =
            include!("test_packets/hwsim_cmd_frame_response_no_cookie.csv");
        assert!(parse_hwsim_cmd(&response_packet2).is_ok());
    }

    #[test]
    fn test_is_mdns_packet() {
        let packet: Vec<u8> = include!("test_packets/hwsim_cmd_frame_mdns.csv");
        let hwsim_msg = HwsimMsg::parse(&packet).unwrap();
        let mdns_frame = Frame::parse(&hwsim_msg).unwrap();
        assert!(!mdns_frame.ieee80211.get_source().is_multicast());
        assert!(mdns_frame.ieee80211.get_destination().is_multicast());
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

    fn build_tx_info_and_compare(frame_bytes: &Bytes, tx_info_expected_bytes: &Bytes) {
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
        let frame_bytes = Bytes::from(include!("test_packets/hwsim_cmd_frame_request.csv"));
        let tx_info_expected_bytes =
            Bytes::from(include!("test_packets/hwsim_cmd_tx_info_response.csv"));
        build_tx_info_and_compare(&frame_bytes, &tx_info_expected_bytes);
    }

    #[test]
    fn test_build_tx_info_and_compare_mdns() {
        let frame_bytes = Bytes::from(include!("test_packets/hwsim_cmd_frame_request_mdns.csv"));
        let tx_info_expected_bytes =
            Bytes::from(include!("test_packets/hwsim_cmd_tx_info_response_mdns.csv"));
        build_tx_info_and_compare(&frame_bytes, &tx_info_expected_bytes);
    }
}
