// Copyright 2025-2026 The Android Open Source Project

use crate::ffi::{AesWrap, DigestType, Hmac};
use crate::ApError;

use netsim_packets::ethernet::MacAddr;
use netsim_packets::ieee80211::eapol::{
    EapolHeader, EapolKeyFrame, EAPOL_KEY_DESC_TYPE_RSN, EAPOL_TYPE_KEY, EAPOL_VERSION,
};
use zerocopy::{FromBytes, IntoBytes};

#[derive(Debug, Clone, Copy, PartialEq)]
enum WpaState {
    Idle,
    PtKStart,       // Sent M1
    PtkNegotiating, // Sent M3
    PtkDone,
}

#[derive(Debug, Clone)]
pub struct WpaAuthenticator {
    bssid: MacAddr,
    sta_addr: MacAddr,
    pmk: Vec<u8>, // Derived from PSK
    anonce: [u8; 32],
    snonce: [u8; 32],
    ptk: Vec<u8>,
    kck: Vec<u8>,
    kek: Vec<u8>,
    tk: Vec<u8>,
    replay_counter: u64,
    state: WpaState,
    rsn_ie: Vec<u8>,
}

const EAPOL_HEADER_LEN: usize = 4;
// KeyDesc(1) + KeyInfo(2) + KeyLen(2) + Replay(8) + Nonce(32) + IV(16) + RSC(8) + ID(8) = 77
const KEY_FRAME_FIXED_LEN: usize = 77;
const MIC_OFFSET: usize = EAPOL_HEADER_LEN + KEY_FRAME_FIXED_LEN; // 81
const MIC_LEN: usize = 16;
const MIC_END: usize = MIC_OFFSET + MIC_LEN; // 97

impl WpaAuthenticator {
    pub fn new(bssid: MacAddr, sta_addr: MacAddr, psk: &[u8], rsn_ie: &[u8]) -> Self {
        // Assume PSK is PMK for now (or derived externally)
        // Usually PMK = PBKDF2(passphrase, ssid, 4096, 32)
        Self {
            bssid,
            sta_addr,
            pmk: psk.to_vec(),
            anonce: [0; 32],
            snonce: [0; 32],
            ptk: Vec::new(),
            kck: Vec::new(),
            kek: Vec::new(),
            tk: Vec::new(),
            replay_counter: 0,
            state: WpaState::Idle,
            rsn_ie: rsn_ie.to_vec(),
        }
    }

    pub fn initiate_handshake(&mut self) -> Result<Vec<u8>, ApError> {
        self.state = WpaState::PtKStart;
        self.replay_counter += 1;

        let anonce_vec = crate::ffi::RandBytes(32);
        self.anonce.copy_from_slice(&anonce_vec);

        // Construct M1
        self.build_eapol_frame(
            0x0080 | 0x0008 | 0x0200, // Key Info: Key Descriptor Version 2 (HMAC-SHA1-128/AES) | Pairwise | Ack (M1 has Ack set)
            &[0u8; 16],               // MIC is 0 in M1
            0,                        // Data Len 0
            &[],
        )
    }
}

#[derive(Debug, Clone)]
pub enum WpaOutput {
    Frame(Vec<u8>),
    InstallKey {
        key_index: u8,
        key: Vec<u8>,
        cipher: String, // "CCMP"
    },
}

impl WpaAuthenticator {
    // ... new ... initiate_handshake ...

    pub fn handle_eapol(&mut self, frame_data: &[u8]) -> Result<Vec<WpaOutput>, ApError> {
        // ...
        // Use zerocopy to parse header
        let (header, body) = match EapolHeader::read_from_prefix(frame_data) {
            Ok(res) => res,
            Err(_) => return Err(ApError::InvalidFrame),
        };

        if header.packet_type != EAPOL_TYPE_KEY {
            return Ok(vec![]);
        }

        // Parse Key Frame
        let (key_frame, _key_data) = match EapolKeyFrame::read_from_prefix(body) {
            Ok(res) => res,
            Err(_) => return Err(ApError::InvalidFrame),
        };

        // ... (snip parsers) ...

        match self.state {
            WpaState::PtKStart | WpaState::Idle => {
                // ... (M2 logic) ...
                self.snonce = key_frame.key_nonce;
                self.calc_ptk();

                if !self.verify_mic(frame_data, &key_frame.mic) {
                    log::warn!("WPA: MIC verification failed for M2 from {}", self.sta_addr);
                    return Ok(vec![]);
                }

                self.state = WpaState::PtkNegotiating;
                self.replay_counter += 1;

                let m3_info = 0x0008 | 0x0040 | 0x0080 | 0x0100 | 0x0200; // Pairwise | Install | Ack | MIC | Secure

                // Construct RSN IE (CCMP-PSK) for M3
                // Use the configured RSN IE to ensure consistency
                let rsn_ie = &self.rsn_ie;

                // Generate Dummy GTK (16 bytes)
                let gtk = crate::ffi::RandBytes(16);
                let gtk_kde = crate::rsn::build_gtk_kde(&gtk, 1); // KeyID 1

                let mut key_data = Vec::new();
                key_data.extend_from_slice(rsn_ie);
                key_data.extend_from_slice(&gtk_kde);

                let mut encrypted_data = Vec::new();
                if !key_data.is_empty() {
                    let mut data_to_wrap = key_data.clone();
                    // Append padding (zeroes) to 8-byte boundary per AES Key Wrap (RFC 3394)
                    while data_to_wrap.len() % 8 != 0 {
                        data_to_wrap.push(0);
                    }
                    encrypted_data = AesWrap(&self.kek, &data_to_wrap);
                }

                log::info!("WPA: Sending M3 to {} (with GTK)", self.sta_addr);

                let m3_bytes = self.build_eapol_frame(
                    m3_info,
                    &[0u8; 16],
                    encrypted_data.len() as u16,
                    &encrypted_data,
                )?;

                let mic = self.calc_mic_for_frame(&m3_bytes);
                let mut final_m3 = m3_bytes;
                final_m3[MIC_OFFSET..MIC_END].copy_from_slice(&mic);

                Ok(vec![WpaOutput::Frame(final_m3)])
            }
            WpaState::PtkNegotiating => {
                if !self.verify_mic(frame_data, &key_frame.mic) {
                    log::warn!("WPA: MIC verification failed for M4 from {}", self.sta_addr);
                    return Ok(vec![]);
                }

                self.state = WpaState::PtkDone;
                log::info!("WPA: Handshake Complete. PTK Installed.");

                // Install Key
                let tk = self.tk.clone();
                Ok(vec![WpaOutput::InstallKey {
                    key_index: 0,
                    key: tk,
                    cipher: "CCMP".to_string(),
                }])
            }
            _ => Ok(vec![]),
        }
    }

    fn calc_ptk(&mut self) {
        // PTK = PRF-384(PMK, "Pairwise key expansion", Min(AA,SA) || Max(AA,SA) || Min(ANonce,SNonce) || Max(ANonce,SNonce))
        let label = b"Pairwise key expansion";
        let mut data = Vec::new();

        let (min_addr, max_addr) = if self.bssid.bytes < self.sta_addr.bytes {
            (&self.bssid.bytes, &self.sta_addr.bytes)
        } else {
            (&self.sta_addr.bytes, &self.bssid.bytes)
        };

        let (min_nonce, max_nonce) = if self.anonce < self.snonce {
            (&self.anonce, &self.snonce)
        } else {
            (&self.snonce, &self.anonce)
        };

        data.extend_from_slice(min_addr);
        data.extend_from_slice(max_addr);
        data.extend_from_slice(min_nonce);
        data.extend_from_slice(max_nonce);

        // PRF-384 (48 bytes)
        let ptk_bytes = self.prf(&self.pmk, label, &data, 384);
        self.ptk = ptk_bytes.clone();

        // Split PTK
        // KCK (16), KEK (16), TK (16)
        self.kck = ptk_bytes[0..16].to_vec();
        self.kek = ptk_bytes[16..32].to_vec();
        self.tk = ptk_bytes[32..48].to_vec();
    }

    fn prf(&self, key: &[u8], label: &[u8], data: &[u8], bit_len: usize) -> Vec<u8> {
        let mut result = Vec::new();
        let bytes_len = bit_len / 8;
        let mut i = 0u8;

        // PRF-X uses HMAC-SHA1
        while result.len() < bytes_len {
            let mut input = Vec::new();
            input.extend_from_slice(label);
            input.push(0);
            input.extend_from_slice(data);
            input.push(i);

            let hmac = Hmac(DigestType::SHA1, &key.to_vec(), &input);
            result.extend_from_slice(&hmac);
            i += 1;
        }
        result.truncate(bytes_len);
        result
    }

    fn calc_mic_for_frame(&self, frame: &[u8]) -> Vec<u8> {
        // MIC is calculated over the EAPol frame with MIC field set to 0.
        // Hmac-SHA1-128 (first 16 bytes of SHA1) for CCMP.
        let mic = Hmac(DigestType::SHA1, &self.kck, &frame.to_vec());
        mic[0..16].to_vec()
    }

    fn verify_mic(&self, frame: &[u8], received_mic: &[u8]) -> bool {
        // To verify, we must zero out the MIC field in the frame, calculate, and compare.
        let mut frame_copy = frame.to_vec();
        // EAPOL Header (4) + Descr(1) + Info(2) + Len(2) + Replay(8) + Nonce(32) + IV(16) + RSC(8) + ID(8) = 81
        // Range 81..97 is MIC.
        if frame_copy.len() < MIC_END {
            return false;
        }

        for i in MIC_OFFSET..MIC_END {
            frame_copy[i] = 0;
        }

        let calculated = self.calc_mic_for_frame(&frame_copy);
        calculated == received_mic
    }

    // Helper to build EAPOL frame
    fn build_eapol_frame(
        &self,
        key_info: u16,
        mic: &[u8],
        data_len: u16,
        key_data: &[u8],
    ) -> Result<Vec<u8>, ApError> {
        let mut frame = Vec::new();

        // EAPOL Packet Body Length: KeyDesc(1) + ... + KeyData(N)
        // KeyFrame is 95 bytes fixed part.
        let body_len = 95 + key_data.len();

        let header = EapolHeader {
            version: EAPOL_VERSION,
            packet_type: EAPOL_TYPE_KEY,
            length: (body_len as u16).to_be_bytes(),
        };

        frame.extend_from_slice(header.as_bytes());

        let key_frame = EapolKeyFrame {
            descriptor_type: EAPOL_KEY_DESC_TYPE_RSN,
            key_info: key_info.to_be_bytes(),
            key_len: 16u16.to_be_bytes(), // TK Len (CCMP=16)
            replay_counter: self.replay_counter.to_be_bytes(),
            key_nonce: self.anonce,
            key_iv: [0; 16],
            key_rsc: [0; 8],
            key_id: [0; 8],
            mic: mic.try_into().unwrap_or([0; 16]),
            key_data_len: data_len.to_be_bytes(),
        };
        frame.extend_from_slice(key_frame.as_bytes());
        frame.extend_from_slice(key_data);

        Ok(frame)
    }
}
