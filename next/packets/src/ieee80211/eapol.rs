// Copyright 2025 The Android Open Source Project

//! EAPOL (Extensible Authentication Protocol over LAN) definitions.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Represents the EAPOL Header.
#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned, Debug, Clone, Copy, Default)]
#[repr(C, packed)]
pub struct EapolHeader {
    /// Protocol Version (e.g., 1 or 2).
    pub version: u8,
    /// Packet Type (e.g., 3 = Key).
    pub packet_type: u8,
    /// Packet Body Length (Big Endian).
    pub length: [u8; 2],
}

impl EapolHeader {
    /// Creates a new EAPOL header.
    pub fn new(version: u8, packet_type: u8, length: u16) -> Self {
        Self { version, packet_type, length: length.to_be_bytes() }
    }
}

/// Represents the EAPOL-Key Frame (used in WPA/WPA2/RSN).
#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EapolKeyFrame {
    /// Key Descriptor Type (1 = RC4, 2 = RSN (WPA2), 254 = WPA).
    pub descriptor_type: u8,
    /// Key Information Flags (Big Endian).
    pub key_info: [u8; 2],
    /// Key Length (Big Endian).
    pub key_len: [u8; 2],
    /// Replay Counter (Big Endian).
    pub replay_counter: [u8; 8],
    /// Key Nonce.
    pub key_nonce: [u8; 32],
    /// Key IV.
    pub key_iv: [u8; 16],
    /// Key RSC.
    pub key_rsc: [u8; 8],
    /// Key ID (Reserved).
    pub key_id: [u8; 8],
    /// Message Integrity Code (MIC).
    pub mic: [u8; 16],
    /// Key Data Length (Big Endian).
    pub key_data_len: [u8; 2],
    // Data follows
}

impl Default for EapolKeyFrame {
    fn default() -> Self {
        Self {
            descriptor_type: 0,
            key_info: [0; 2],
            key_len: [0; 2],
            replay_counter: [0; 8],
            key_nonce: [0; 32],
            key_iv: [0; 16],
            key_rsc: [0; 8],
            key_id: [0; 8],
            mic: [0; 16],
            key_data_len: [0; 2],
        }
    }
}

impl EapolKeyFrame {
    /// Creates a new EAPOL-Key frame with common defaults.
    pub fn new(
        descriptor_type: u8,
        key_info: u16,
        key_len: u16,
        replay_counter: u64,
        key_nonce: [u8; 32],
        key_iv: [u8; 16],
        key_rsc: u64,
        mic: [u8; 16],
        key_data_len: u16,
    ) -> Self {
        Self {
            descriptor_type,
            key_info: key_info.to_be_bytes(),
            key_len: key_len.to_be_bytes(),
            replay_counter: replay_counter.to_be_bytes(),
            key_nonce,
            key_iv,
            key_rsc: key_rsc.to_be_bytes(),
            key_id: [0; 8], // Reserved
            mic,
            key_data_len: key_data_len.to_be_bytes(),
        }
    }
}

/// EAPOL Version 1.
pub const EAPOL_VERSION: u8 = 1;
/// EAPOL Packet Type: Key.
pub const EAPOL_TYPE_KEY: u8 = 3;
/// Key Descriptor Type: RSN (WPA2).
pub const EAPOL_KEY_DESC_TYPE_RSN: u8 = 2;
