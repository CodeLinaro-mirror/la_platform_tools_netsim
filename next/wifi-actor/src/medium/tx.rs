// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use bytes::Bytes;
use netsim_packets::{FrameDirection, HwsimFrame, HwsimMsg, Ieee80211};
use tracing::debug;

use crate::{
    error::WifiError,
    medium::{
        core::Medium,
        types::{Station, WifiResult},
        utils::{self, build_tx_info},
    },
};

// Packets flowing from Medium to Guest (Destination)
impl Medium {
    /// Encodes a HwsimMsg and pushes it to the output queue.
    ///
    /// This helper simplifies the error handling and queue management for
    /// outgoing packets.
    fn push_packet(
        &mut self,
        client_id: u32,
        msg: &HwsimMsg,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        let packet =
            msg.encode_to_vec().map_err(|e| WifiError::Internal(Box::from(e.to_string())))?.into();
        out_queue.push((client_id, packet));
        Ok(())
    }

    /// Resolves targets for a given destination address.
    ///
    /// - If unicast and known: returns the specific station.
    /// - If multicast: returns all subscribed stations.
    /// - If unknown: returns empty list.
    fn resolve_targets(&self, dest_addr: &netsim_packets::MacAddress) -> Vec<Station> {
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
        } else {
            // Fallback: Android DHCP frames systematically preserve hwsimaddr inside
            // payloads. Slirp returns these dynamically forcing Destination MAC
            // outside standard Random MAC arrays.
            for station in self.stations.values() {
                if &station.hwsim_addr == dest_addr {
                    targets.push(station.clone());
                    break;
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

    /// Processes an IEEE 802.3 packet (Ethernet frame), typically from Slirp or
    /// a Tun interface.
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
        // Try to determine the destination MacAddress from the Ethernet header
        let dest_mac_bytes: [u8; 6] = packet[0..6].try_into().map_err(|_| {
            WifiError::Internal(Box::from("Failed to parse Ethernet Destination MAC"))
        })?;
        let dest_mac = netsim_packets::MacAddress::new(dest_mac_bytes);

        if dest_mac.is_broadcast() || dest_mac.is_multicast() {
            let bssids = self.key_store.bssids.read().unwrap().clone();
            for bssid in bssids {
                let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Ok(ieee80211) = netsim_packets::Ieee80211::from_ieee8023_qos(
                    packet,
                    bssid,
                    FrameDirection::FromAp,
                    true,
                    seq,
                ) {
                    let _ = self.route_infra_packet(ieee80211, out_queue);
                }
            }
            return Ok(());
        }

        let Some(bssid) = self.key_store.get_station_bssid(&dest_mac) else {
            tracing::warn!("Dropping unicast packet to unknown station {}", dest_mac);
            return Ok(());
        };

        let seq = self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ieee80211 = netsim_packets::Ieee80211::from_ieee8023_qos(
            packet,
            bssid,
            FrameDirection::FromAp,
            true,
            seq,
        )
        .map_err(|e| WifiError::Internal(Box::from(e)))?;
        self.route_infra_packet(ieee80211, out_queue)
    }

    /// Entry point for transmitting raw IEEE 802.11 bytes from the
    /// infrastructure.
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
            WifiError::Internal(Box::from(format!("Failed to process IEEE 802.11 response: {e}")))
        })?;
        self.route_infra_packet(ieee80211, out_queue)
    }

    /// Routes a decoded IEEE 802.11 frame from the infrastructure to the
    /// appropriate stations.
    ///
    /// Handles encryption (if applicable), resolves targets, and delivers the
    /// packet.
    pub(crate) fn route_infra_packet(
        &mut self,
        ieee80211: Ieee80211,
        out_queue: &mut Vec<(u32, Bytes)>,
    ) -> WifiResult<()> {
        let dest_addr = ieee80211.get_destination();
        let mut targets = self.resolve_targets(&dest_addr);

        if targets.is_empty() && !dest_addr.is_multicast() {
            // Unknown Unicast Flooding: Deliver to all enabled stations
            for station in self.stations.values() {
                targets.push(station.clone());
            }
        }

        let is_m2u_conversion = (targets.len() > 1 || dest_addr.is_multicast())
            && !self.key_store.bssids.read().unwrap().is_empty();

        for dest in targets {
            if self.enabled(dest.client_id)? {
                let frame_to_send = if is_m2u_conversion {
                    let mut unicast_frame = ieee80211.clone();
                    let target_mac = netsim_packets::MacAddress::new(dest.addr.into());
                    unicast_frame.set_destination(&target_mac);

                    // If WPA is active, we must successfully encrypt using the destination's PTK
                    // for M2U (Broadcast-to-Unicast) to be valid.
                    if let Some(encrypted_bytes) = self.key_store.try_encrypt(&unicast_frame) {
                        Ieee80211::decode(&encrypted_bytes).unwrap_or(unicast_frame)
                    } else if dest_addr.is_multicast() || dest_addr.is_broadcast() {
                        // For Broadcast/Multicast M2U, if encryption fails (keys not ready),
                        // we MUST NOT send unencrypted unicast. Fall back to original.
                        ieee80211.clone()
                    } else {
                        // For Unicast Flooding, we keep the rewritten MAC even if unencrypted
                        // as per standard 802.11 bridge behavior.
                        unicast_frame
                    }
                } else if let Some(encrypted_bytes) = self.key_store.try_encrypt(&ieee80211) {
                    Ieee80211::decode(&encrypted_bytes).unwrap_or(ieee80211.clone())
                } else {
                    ieee80211.clone()
                };

                let msg = utils::create_hwsim_msg_from_frame(
                    &frame_to_send,
                    &dest.hwsim_addr,
                    dest.freq,
                    None,
                )?;
                self.wifi_stats.incr_hwsim_frames_tx();
                self.push_packet(dest.client_id, &msg, out_queue)?;
                self.incr_rx(dest.client_id)?;
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
            return Err(WifiError::Internal(Box::from(format!(
                "Dropped packet from {} to {}",
                source.addr, dest_addr
            ))));
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

        let is_m2u_conversion = targets.len() > 1 || dest_addr.is_multicast();

        for dest in targets {
            // Drop unicast packets destined to the sender itself (invalid for hwsim)
            // But ALLOW multicast packets to reach the sender (AP Reflection) if enabled
            if dest.addr == source.addr
                && (!dest_addr.is_multicast() || !self.simulate_ap_reflection)
            {
                continue;
            }
            let src_enabled = self.enabled(source.client_id)?;
            let dst_enabled = self.enabled(dest.client_id)?;
            if src_enabled && dst_enabled {
                let mut target_frame = ieee80211.clone();
                if is_m2u_conversion && dest.addr != source_addr {
                    let target_mac = netsim_packets::MacAddress::new(dest.addr.into());
                    target_frame.set_destination(&target_mac);
                }

                match utils::create_encrypted_hwsim_msg(
                    frame,
                    &target_frame,
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
