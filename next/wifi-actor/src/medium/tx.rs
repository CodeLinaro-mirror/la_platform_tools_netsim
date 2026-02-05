// Copyright 2025 The Android Open Source Project

use crate::error::WifiError;
use crate::medium::core::Medium;
use crate::medium::types::{Station, WifiResult};
use crate::medium::utils::{self, build_tx_info};
use bytes::Bytes;
use log::debug;
use netsim_packets::ieee80211::{FrameDirection, Ieee80211};
use netsim_packets::netlink::hwsim_frame::HwsimFrame;
use netsim_packets::netlink::HwsimMsg;
use std::collections::HashSet;

// Packets flowing from Medium to Guest (Destination)
impl Medium {
    /// Encodes a HwsimMsg and pushes it to the output queue.
    ///
    /// This helper simplifies the error handling and queue management for outgoing packets.
    fn push_packet(
        &mut self,
        client_id: u32,
        msg: &HwsimMsg,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        let packet = msg.encode_to_vec().map_err(|e| WifiError::Internal(e.to_string()))?.into();
        out_queue.push((client_id, packet));
        Ok(())
    }

    /// Resolves targets for a given destination address.
    ///
    /// - If unicast and known: returns the specific station.
    /// - If multicast: returns all subscribed stations.
    /// - If unknown: returns empty list.
    fn resolve_targets(&self, dest_addr: &netsim_packets::ieee80211::MacAddress) -> Vec<Station> {
        let mut targets = Vec::new();
        if self.contains_station(dest_addr) {
            if let Ok(station) = self.get_station(dest_addr) {
                targets.push(station.clone());
            }
        } else if dest_addr.is_multicast() {
            let mut visited = HashSet::new();
            for station in self.stations.values() {
                if visited.insert((station.hwsim_addr, station.freq)) {
                    targets.push(station.clone());
                }
            }
        }
        targets
    }

    pub fn ack_frame(
        &mut self,
        client_id: u32,
        frame: &HwsimFrame,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        let tx_info = build_tx_info(&frame.hwsim_msg)?;
        self.push_packet(client_id, &tx_info, out_queue)?;
        self.incr_tx(client_id)
    }

    /// Processes an IEEE 802.3 packet (Ethernet frame), typically from Slirp or a Tun interface.
    ///
    /// Converts the Ethernet frame to an IEEE 802.11 frame and routes it.
    pub fn process_ieee8023_response(
        &mut self,
        packet: &Bytes,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        if self.debug.debug_no_traffic || self.debug.debug_no_network {
            return Ok(());
        }
        // TODO: Support multiple APs (BSSIDs).
        // Currently, we assume a single BSSID globally.
        // To support multiple APs, we need to store which BSSID each Station is associated with
        // (e.g. Map<StationMAC, BSSID>) and look it up here using the packet's Destination MAC.
        let bssid = self
            .key_store
            .get_bssid()
            .unwrap_or(netsim_packets::ieee80211::MacAddress::new([0, 0, 0, 0, 0, 0]));

        let ieee80211 =
            Ieee80211::from_ieee8023(packet, bssid, FrameDirection::FromAp).map_err(|e| {
                WifiError::Internal(format!("Failed to process IEEE 802.3 response: {e}"))
            })?;
        self.route_infra_packet(ieee80211, out_queue)
    }

    /// Entry point for transmitting raw IEEE 802.11 bytes from the infrastructure.
    ///
    /// Decodes the packet and delegates to `route_infra_packet`.
    pub fn transmit_from_infra(
        &mut self,
        packet: &Bytes,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        if self.debug.debug_no_traffic {
            return Ok(());
        }
        let ieee80211 = Ieee80211::decode_full(packet).map_err(|e| {
            WifiError::Internal(format!("Failed to process IEEE 802.11 response: {e}"))
        })?;
        self.route_infra_packet(ieee80211, out_queue)
    }

    /// Routes a decoded IEEE 802.11 frame from the infrastructure to the appropriate stations.
    ///
    /// Handles encryption (if applicable), resolves targets, and delivers the packet.
    pub(crate) fn route_infra_packet(
        &mut self,
        mut ieee80211: Ieee80211,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        if let Some(encrypted_bytes) = self.key_store.try_encrypt(&ieee80211) {
            ieee80211 = Ieee80211::decode(&encrypted_bytes).map_err(|e| {
                WifiError::Internal(format!("Failed to decode encrypted frame: {e}"))
            })?;
        }
        let dest_addr = ieee80211.get_destination();
        log::debug!(
            "Medium: Routing Infra Packet. Dest: {}, Source: {}",
            dest_addr,
            ieee80211.get_source()
        );
        let mut targets = self.resolve_targets(&dest_addr);

        if targets.is_empty() && !dest_addr.is_multicast() {
            // Unknown Unicast Flooding: Deliver to all enabled stations
            debug!(
                "Flooding frame to unknown destination: {dest_addr} (Potential DHCP CHADDR Trap)"
            );
            for station in self.stations.values() {
                targets.push(station.clone());
            }
        }

        let is_flooding = targets.len() > 1 && !dest_addr.is_multicast();

        for dest in targets {
            if self.enabled(dest.client_id)? {
                let mut frame_to_send = ieee80211.clone();
                if is_flooding {
                    // Rewrite Destination MAC to match station's MAC
                    // ensuring the Guest kernel accepts the packet.
                    let target_mac = netsim_packets::ieee80211::MacAddress::new(
                        dest.addr.try_into().unwrap_or([0; 6]),
                    );
                    debug!(
                        "Rewriting Destination MAC for flood: {} -> {} (Target: Client {})",
                        dest_addr, target_mac, dest.client_id
                    );
                    frame_to_send.set_destination(&target_mac);
                }

                let msg = utils::create_hwsim_msg_from_frame(
                    &frame_to_send,
                    &dest.hwsim_addr,
                    dest.freq,
                    None,
                )?;
                self.wifi_stats.incr_hwsim_frames_tx();
                self.push_packet(dest.client_id, &msg, out_queue)?;
                self.incr_rx(dest.client_id)?;
            } else {
                debug!("Dropping frame to disabled client {}", dest.client_id);
            }
        }
        Ok(())
    }

    pub fn transmit(
        &mut self,
        frame: &HwsimFrame,
        ieee80211: &Ieee80211,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        let source_addr = ieee80211.get_source();
        let source = self.get_station(&source_addr)?.clone();
        let dest_addr = ieee80211.get_destination();

        let targets = self.resolve_targets(&dest_addr);
        if targets.is_empty() && !dest_addr.is_multicast() {
            return Err(WifiError::Internal(format!(
                "Dropped packet from {} to {}",
                source.addr, dest_addr
            )));
        }

        if dest_addr.is_multicast() {
            debug!("Frame multicast {ieee80211}");
            if dest_addr.is_mdns() {
                self.wifi_stats.incr_mdns_count();
            }
        } else {
            debug!("Frame deliver unicast from {} to {}", source.addr, dest_addr);
            self.wifi_stats.incr_wmedium_unicast_frames_tx();
        }

        for dest in targets {
            if dest.addr == source.addr {
                continue;
            }
            let src_enabled = self.enabled(source.client_id)?;
            let dst_enabled = self.enabled(dest.client_id)?;
            if src_enabled && dst_enabled {
                match utils::create_encrypted_hwsim_msg(
                    frame,
                    ieee80211,
                    &dest.hwsim_addr,
                    &self.key_store,
                    self.simulate_ap_reflection,
                ) {
                    Ok(msg) => {
                        self.wifi_stats.incr_wmedium_frames_tx();
                        self.wifi_stats.incr_hwsim_frames_tx();
                        self.push_packet(dest.client_id, &msg, out_queue)?;
                        self.incr_rx(dest.client_id)?;
                    }
                    Err(e) => self.wifi_stats.log_and_incr_err_count(&e),
                }
            }
        }
        Ok(())
    }
}
