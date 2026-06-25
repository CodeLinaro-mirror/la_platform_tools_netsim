// Copyright 2025-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use netsim_packets::{CcmpHeader, Ieee80211, MacAddress};
use tracing::error;
use zerocopy::IntoBytes;

use crate::ffi::{AesCcmDecrypt, AesCcmEncrypt};

const CCMP_HDR_LEN: usize = 8;

#[derive(Debug)]
pub struct SessionKeys {
    pub tk: Vec<u8>,
    pub tx_pn: AtomicU64,
}

impl Default for SessionKeys {
    fn default() -> Self {
        Self { tk: Vec::new(), tx_pn: AtomicU64::new(1) }
    }
}

#[derive(Clone, Debug)]
pub struct SharedKeyStore {
    // Set of all active AP BSSIDs
    pub bssids: Arc<RwLock<std::collections::HashSet<MacAddress>>>,
    // Map of Station Address -> Connected BSSID
    pub station_bssids: Arc<RwLock<HashMap<MacAddress, MacAddress>>>,
    // Map of Station Address -> SessionKeys
    pub sessions: Arc<RwLock<HashMap<MacAddress, Arc<SessionKeys>>>>,
    pub gtks: Arc<RwLock<HashMap<MacAddress, [u8; 16]>>>,
    pub gtk_tx_pn: Arc<AtomicU64>,
}

impl Default for SharedKeyStore {
    fn default() -> Self {
        Self {
            bssids: Arc::new(RwLock::new(std::collections::HashSet::new())),
            station_bssids: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            gtks: Arc::new(RwLock::new(HashMap::new())),
            gtk_tx_pn: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl SharedKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_bssid(&self, bssid: MacAddress) {
        self.bssids.write().unwrap().insert(bssid);
    }

    pub fn has_bssid(&self, bssid: &MacAddress) -> bool {
        self.bssids.read().unwrap().contains(bssid)
    }

    pub fn set_station_bssid(&self, sta_addr: MacAddress, bssid: MacAddress) {
        self.station_bssids.write().unwrap().insert(sta_addr, bssid);
    }

    pub fn get_station_bssid(&self, sta_addr: &MacAddress) -> Option<MacAddress> {
        self.station_bssids.read().unwrap().get(sta_addr).copied()
    }

    pub fn add_session(&self, sta_addr: MacAddress, tk: Vec<u8>) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(sta_addr, Arc::new(SessionKeys { tk, tx_pn: AtomicU64::new(1) }));
    }

    pub fn get_gtk(&self, bssid: &MacAddress) -> Option<[u8; 16]> {
        self.gtks.read().unwrap().get(bssid).copied()
    }

    pub fn set_gtk(&self, bssid: MacAddress, gtk: [u8; 16]) {
        self.gtks.write().unwrap().insert(bssid, gtk);
    }

    pub fn remove_session(&self, sta_addr: &MacAddress) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(sta_addr);
    }

    pub fn remove_station_bssid(&self, sta_addr: &MacAddress) {
        self.station_bssids.write().unwrap().remove(sta_addr);
    }

    pub fn try_encrypt(&self, ieee80211: &Ieee80211) -> Option<Vec<u8>> {
        let dest = ieee80211.get_destination();
        let is_group = dest.is_multicast() || dest.is_broadcast();

        let (tk, pn, key_id) = if is_group {
            let bssid = ieee80211.get_bssid()?;
            let gtks = self.gtks.read().ok()?;
            let gtk = gtks.get(&bssid)?;
            let pn = self.gtk_tx_pn.fetch_add(1, Ordering::SeqCst);
            (gtk.to_vec(), pn, 1) // KeyID 1
        } else {
            let sessions = self.sessions.read().ok()?;
            let session = sessions.get(&dest)?;
            let pn = session.tx_pn.fetch_add(1, Ordering::SeqCst);
            (session.tk.clone(), pn, 0) // KeyID 0
        };

        if tk.is_empty() {
            return None;
        }

        let mut nonce = [0u8; 13];
        nonce[0] = 0; // Priority (0)
        nonce[1..7].copy_from_slice(&ieee80211.get_addr2().bytes); // A2 (Src/BSSID)
        // PN (6 bytes)
        let pn_bytes = pn.to_le_bytes();
        // CCMP Nonce: Priority(1) || A2(6) || PN(6)

        // Hostapd-rs way:
        nonce[7] = pn_bytes[5];
        nonce[8] = pn_bytes[4];
        nonce[9] = pn_bytes[3];
        nonce[10] = pn_bytes[2];
        nonce[11] = pn_bytes[1];
        nonce[12] = pn_bytes[0];
        // PN is Little Endian from to_le_bytes (LSB at index 0).
        // We map to Nonce bytes [7..13] matching legacy hostapd logic.

        // The CCMP AAD explicitly requires the 'Protected' frame control bit to be
        // active. We must calculate the AAD against the final physical MAC byte
        // sequence!
        let mut final_fc_bytes = ieee80211.as_bytes().to_vec();
        final_fc_bytes[1] |= 0x40; // Force Protected Bit inside the temporary buffer
        let final_ieee = Ieee80211::decode(&final_fc_bytes).unwrap();
        let aad = final_ieee.get_aad();

        let payload = ieee80211.get_payload();

        let mut ciphertext = Vec::with_capacity(payload.len() + 8);
        let success = AesCcmEncrypt(&tk[..16], &nonce, &aad, &payload, &mut ciphertext, 8);
        if !success {
            return None;
        }

        // Reconstruct Frame
        let mut new_packet =
            Vec::with_capacity(ieee80211.hdr_length() + CCMP_HDR_LEN + ciphertext.len());
        new_packet.extend_from_slice(&ieee80211.as_bytes()[..ieee80211.hdr_length()]);

        // CCMP Header
        // PN0, PN1, Rsvd, KeyID_ExtIV, PN2, PN3, PN4, PN5
        let ccmp_key_id = 0x20 | (key_id << 6);
        let ccmp_header = CcmpHeader {
            pn0: pn_bytes[0],
            pn1: pn_bytes[1],
            rsvd: 0,
            key_id: ccmp_key_id,
            pn2: pn_bytes[2],
            pn3: pn_bytes[3],
            pn4: pn_bytes[4],
            pn5: pn_bytes[5],
        };
        new_packet.extend_from_slice(ccmp_header.as_bytes());

        new_packet.extend_from_slice(&ciphertext);

        // Set Protected Bit
        new_packet[1] |= 0x40;

        Some(new_packet)
    }

    pub fn try_decrypt(&self, ieee80211: &Ieee80211) -> Option<Vec<u8>> {
        // Decrypt if source is in sessions (Unicast from Station)
        let src = ieee80211.get_source();
        let sessions = self.sessions.read().ok()?;
        let session = sessions.get(&src)?;

        if ieee80211.as_bytes().len() < ieee80211.hdr_length() + CCMP_HDR_LEN {
            return None;
        }

        // Extract Nonce from Frame (CCMP Header)
        // Frame: Header || CCMP Header || Data || MIC
        let hdr_len = ieee80211.hdr_length();
        let ccmp_hdr = &ieee80211.as_bytes()[hdr_len..hdr_len + 8];
        let pn0 = ccmp_hdr[0];
        let pn1 = ccmp_hdr[1];
        let pn2 = ccmp_hdr[4];
        let pn3 = ccmp_hdr[5];
        let pn4 = ccmp_hdr[6];
        let pn5 = ccmp_hdr[7];

        let mut nonce = [0u8; 13];
        nonce[0] = 0; // Priority
        nonce[1..7].copy_from_slice(&ieee80211.get_addr2().bytes); // A2 (Src/STA)
        nonce[7] = pn5;
        nonce[8] = pn4;
        nonce[9] = pn3;
        nonce[10] = pn2;
        nonce[11] = pn1;
        nonce[12] = pn0;

        // Payload
        let data = &ieee80211.as_bytes()[hdr_len + CCMP_HDR_LEN..];
        let aad = ieee80211.get_aad();

        if data.len() < 8 {
            return None;
        }
        let mut out_plain = Vec::with_capacity(data.len() - 8);
        let success = AesCcmDecrypt(&session.tk[..16], &nonce, &aad, data, &mut out_plain, 8);
        if !success {
            error!(
                "CCMP DECRYPT FAILED! hdr_len: {}, msg_len: {}, aad_len: {}",
                hdr_len,
                data.len(),
                aad.len()
            );
            error!("RAW FRAME TO DECRYPT (Hex): {:02X?}", ieee80211.as_bytes());
            return None;
        }
        let plaintext = out_plain;

        let mut new_packet = Vec::new();
        new_packet.extend_from_slice(&ieee80211.as_bytes()[..hdr_len]);
        new_packet.extend_from_slice(&plaintext);

        new_packet[1] &= !0x40; // Clear protected bit
        Some(new_packet)
    }
}
