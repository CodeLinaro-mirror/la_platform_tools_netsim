// Copyright 2025 The Android Open Source Project

use crate::error::WifiError;
use crate::medium::core::Medium;
use crate::medium::tx_packet_state::{InfraTarget, TxPacketState};
use crate::medium::{utils, WifiResult};
use bytes::Bytes;
use log::debug;
use netsim_packets::ieee80211::{DataSubType, Ieee80211};

// Packets flowing from Guest (Source) to Medium
impl Medium {
    /// Main entry point for processing RX packets from a guest (radio).
    ///
    /// # Flow
    /// 1. **Parse & Stats**: Validates `HwsimFrame` and updates RX statistics.
    /// 2. **Station Tracking**: Updates the known state (frequency, etc.) of the source station.
    /// 3. **Decryption**: Attempts to decrypt the frame if a key session exists.
    /// 4. **Routing**: Determines where the packet should go (`ap`, `slirp`, `stations`).
    pub fn resolve_tx_packet(
        &mut self,
        client_id: u32,
        packet: &Bytes,
    ) -> WifiResult<TxPacketState> {
        // 1. Parse & Stats
        let frame = utils::parse_hwsim_frame(packet, client_id).map_err(|e| {
            WifiError::Internal(format!("error validate for client {client_id}: {e}"))
        })?;

        if self.debug.debug_no_traffic {
            return Ok(TxPacketState {
                infra_target: InfraTarget::None,
                stations: false,
                frame,
                plaintext_ieee80211: None,
                plaintext_bytes: None,
            });
        }

        self.wifi_stats.incr_hwsim_frames_rx();

        // 2. Station Tracking
        self.upsert_station(client_id, &frame).map_err(|e| {
            WifiError::Internal(format!("error upsert station for client {client_id}: {e}"))
        })?;

        if let Some(freq) = frame.attrs.freq {
            self.get_station_mut(&frame.ieee80211.get_source())
                .map(|sta| sta.update_freq(freq))
                .map_err(|e| self.wifi_stats.log_and_incr_err_count(&e))
                .ok();
        }

        // 3. Decryption
        let (plaintext_bytes, plaintext_ieee80211) = self.try_decrypt_frame(&frame.ieee80211);

        // 4. Routing
        let mut tx_state = TxPacketState {
            infra_target: InfraTarget::None,
            stations: false,
            frame,
            plaintext_ieee80211,
            plaintext_bytes,
        };

        // If we have plaintext, use it for routing decisions. Otherwise use the original frame (likely Mgmt/Open).
        let routing_frame = tx_state.get_ieee80211();

        if routing_frame.is_mgmt() {
            self.wifi_stats.incr_mgmt_frames_rx();
        }

        let (infra_target, stations) = self.determine_routes(client_id, routing_frame);
        tx_state.infra_target = infra_target;
        tx_state.stations = stations;

        Ok(tx_state)
    }

    /// Attempts to decrypt the 802.11 frame using the shared key store.
    fn try_decrypt_frame(&self, ieee80211: &Ieee80211) -> (Option<Bytes>, Option<Ieee80211>) {
        if let Some(bytes) = self.key_store.try_decrypt(ieee80211) {
            match Ieee80211::decode(&bytes) {
                Ok(parsed) => (Some(Bytes::from(bytes)), Some(parsed)),
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        }
    }

    /// Routes the packet to the appropriate destinations (AP, Slirp, Stations).
    ///
    /// Returns `(InfraTarget, stations)`.
    fn determine_routes(&mut self, client_id: u32, ieee80211: &Ieee80211) -> (InfraTarget, bool) {
        let dest_addr = ieee80211.get_destination();
        let ap_bssid = self.key_store.get_bssid();

        let mut infra_target = InfraTarget::None;
        let mut stations = false;

        // --- Stations (Loopback / Peer-to-Peer) ---
        if self.contains_station(&dest_addr) {
            if !self.debug.debug_no_wmedium {
                stations = true;
            }
            return (infra_target, stations);
        }

        if dest_addr.is_multicast()
            && !(self.debug.debug_no_wmedium
                || (self.debug.debug_no_mdns_wmedium && dest_addr.is_mdns()))
        {
            stations = true;
        }

        // --- AP & Slirp ---

        // Check BSSID match for Infrastructure frames
        if let Some(bssid) = ieee80211.get_bssid() {
            if !bssid.is_multicast() && Some(bssid) != ap_bssid {
                return (infra_target, stations);
            }
        }

        if ieee80211.is_data() {
            // Data Frames
            let is_eapol = ieee80211.is_eapol().unwrap_or_else(|e| {
                debug!("Failed to get ether type for is_eapol(): {e}");
                false
            });

            if is_eapol {
                infra_target = InfraTarget::Ap;
            } else if ieee80211.is_to_ap() {
                // ToDS frames (Station -> AP)
                if ieee80211.stype() != u8::from(DataSubType::Nodata) {
                    // Check if should go to Slirp
                    if self.debug.debug_no_network
                        || (self.debug.debug_no_guest_to_host_mdns && dest_addr.is_mdns())
                    {
                        infra_target = InfraTarget::None;
                    } else if self.enabled(client_id).unwrap_or(false) {
                        infra_target = InfraTarget::Slirp;
                    } else {
                        // If client disabled, only multicast or own-BSSID traffic?
                        if dest_addr.is_multicast() || Some(dest_addr) == ap_bssid {
                            infra_target = InfraTarget::Slirp;
                        }
                    };
                }
            }
        } else {
            // Management or Control Frames
            let addr1 = ieee80211.get_addr1(); // Destination/RA
            if addr1.is_multicast() || addr1.is_broadcast() || Some(addr1) == ap_bssid {
                infra_target = InfraTarget::Ap;
            }
        }

        (infra_target, stations)
    }
}
