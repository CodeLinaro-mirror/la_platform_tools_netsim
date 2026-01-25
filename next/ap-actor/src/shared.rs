// Copyright 2025-2026 The Android Open Source Project

use aes::Aes128;
use ccm::{
    aead::{Aead, KeyInit, Payload},
    consts::{U13, U8},
    Ccm,
};
use netsim_packets::ieee80211::{CcmpHeader, Ieee80211, MacAddress};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};
use zerocopy::IntoBytes;

type AesCcm = Ccm<Aes128, U8, U13>;
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

#[derive(Clone, Debug, Default)]
pub struct SharedKeyStore {
    // Current BSSID of the AP
    pub bssid: Arc<RwLock<Option<MacAddress>>>,
    // Map of Station Address -> SessionKeys
    pub sessions: Arc<RwLock<HashMap<MacAddress, Arc<SessionKeys>>>>,
}

impl SharedKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_bssid(&self, bssid: MacAddress) {
        *self.bssid.write().unwrap() = Some(bssid);
    }

    pub fn get_bssid(&self) -> Option<MacAddress> {
        *self.bssid.read().unwrap()
    }

    pub fn add_session(&self, sta_addr: MacAddress, tk: Vec<u8>) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(sta_addr, Arc::new(SessionKeys { tk, tx_pn: AtomicU64::new(1) }));
    }

    pub fn remove_session(&self, sta_addr: &MacAddress) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(sta_addr);
    }

    pub fn try_encrypt(&self, ieee80211: &Ieee80211) -> Option<Vec<u8>> {
        let sessions = self.sessions.read().ok()?;
        // Encrypt if destination is in sessions (Unicast to Station)
        let dest = ieee80211.get_destination();
        let session = sessions.get(&dest)?;

        let pn = session.tx_pn.fetch_add(1, Ordering::SeqCst);
        let key = ccm::aead::generic_array::GenericArray::from_slice(&session.tk[..16]);
        let cipher = AesCcm::new(key);

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
        // Wait, to_le_bytes puts LSB at index 0.
        // If pn=1, bytes=[1,0,0,0,0,0,0,0].
        // nonce[12] = 1, nonce[7] = 0.
        // Seems correct order for nonce construction if it matches hostapd-rs.

        let nonce_ga = ccm::aead::generic_array::GenericArray::from_slice(&nonce);

        // AAD
        // AAD
        let aad = ieee80211.get_aad();

        let payload = ieee80211.get_payload(); // Assuming get_payload

        let ciphertext = match cipher.encrypt(nonce_ga, Payload { msg: &payload, aad: &aad }) {
            Ok(c) => c,
            Err(_) => return None,
        };

        // Reconstruct Frame
        let mut new_packet =
            Vec::with_capacity(ieee80211.hdr_length() + CCMP_HDR_LEN + ciphertext.len());
        new_packet.extend_from_slice(&ieee80211.as_bytes()[..ieee80211.hdr_length()]);

        // CCMP Header
        // PN0, PN1, Rsvd, KeyID_ExtIV, PN2, PN3, PN4, PN5
        let ccmp_header = CcmpHeader {
            pn0: pn_bytes[0],
            pn1: pn_bytes[1],
            rsvd: 0,
            key_id: 0x20, // KeyID 0 + ExtIV (bit 5 set)
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

        let key = ccm::aead::generic_array::GenericArray::from_slice(&session.tk[..16]);
        let cipher = AesCcm::new(key);

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

        let nonce_ga = ccm::aead::generic_array::GenericArray::from_slice(&nonce);

        // Payload
        let data = &ieee80211.as_bytes()[hdr_len + CCMP_HDR_LEN..];
        let aad = ieee80211.get_aad();

        let plaintext = match cipher.decrypt(nonce_ga, Payload { msg: data, aad: &aad }) {
            Ok(p) => p,
            Err(_) => return None,
        };

        let mut new_packet = Vec::new();
        new_packet.extend_from_slice(&ieee80211.as_bytes()[..hdr_len]);
        new_packet.extend_from_slice(&plaintext);

        new_packet[1] &= !0x40; // Clear protected bit
        Some(new_packet)
    }
}
