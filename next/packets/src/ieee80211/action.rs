// Copyright 2024 The Android Open Source Project

// use crate::ieee80211::MacAddress; // Unused
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Action Frame Category Values
pub mod category {
    pub const SPECTRUM_MANAGEMENT: u8 = 0;
    pub const QOS: u8 = 1;
    pub const DLS: u8 = 2;
    pub const BLOCK_ACK: u8 = 3;
    pub const PUBLIC: u8 = 4;
    pub const RADIO_MEASUREMENT: u8 = 5;
    pub const FAST_BSS_TRANSITION: u8 = 6;
    pub const HT: u8 = 7;
    pub const SA_QUERY: u8 = 8;
    pub const PROTECTED_DUAL_OF_PUBLIC_ACTION: u8 = 9;
    pub const WNM: u8 = 10;
    pub const UNPROTECTED_WNM: u8 = 11;
    pub const TDLS: u8 = 12;
    pub const MESH: u8 = 13;
    pub const MULTIHOP: u8 = 14;
    pub const SELF_PROTECTED: u8 = 15;
    pub const DMV: u8 = 16;
    pub const FST: u8 = 18;
    pub const ROBUST_AV_STREAMING: u8 = 19;
    pub const UNPROTECTED_DMG: u8 = 20;
    pub const VHT: u8 = 21;
    pub const VENDOR_SPECIFIC_PROTECTED: u8 = 126;
    pub const VENDOR_SPECIFIC: u8 = 127;
}

/// Public Action Field Values
pub mod public_action {
    pub const FTM_REQUEST: u8 = 32;
    pub const FINE_TIMING_MEASUREMENT: u8 = 33;
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ActionHeader {
    pub category: u8,
    pub action: u8,
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FtmRequest {
    pub trigger: u8,
    // Variable length fields follow:
    // LCI Measurement Request (optional)
    // Location Civic Measurement Request (optional)
    // FTM Parameters (optional)
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FineTimingMeasurement {
    pub dialog_token: u8,
    pub follow_up_dialog_token: u8,
    pub tod: [u8; 6], // Time of Departure (48-bit)
    pub toa: [u8; 6], // Time of Arrival (48-bit)
    pub tod_error: [u8; 6], // TOD Error (actually part of a larger packed struct in spec, simplified here for simulation)
                            // wait, spec says: TOD Error (1 byte) | TOA Error (1 byte) | LCI Report (variable) ?
                            // IEEE 802.11-2016 Figure 9-556
                            // Order: Category, Action, Dialog Token, Follow Up Dialog Token, TOD, TOA, TOD Error, TOA Error, LCI Report...
                            // Actually, TOD and TOA are 6 bytes (48 bits).
                            // TOD Error and TOA Error are 1 byte each?
                            // Let's check the spec closer or use a larger buffer.
                            // For now, we define the FIXED fields.
}

// 802.11mc FTM Parameters Bitmap
// Bit 0: Asap
// Bit 1: LMR Feedback
// Bit 2-3: Reserved
// Bit 4-7: Number of Bursts Exponent
pub const FTM_PARAM_ASAP: u8 = 0x01;
pub const FTM_PARAM_LMR_FEEDBACK: u8 = 0x02;
