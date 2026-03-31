// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use zerocopy::{
    byteorder::LittleEndian, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, U16,
};

use crate::{
    ethernet::MacAddr,
    ieee80211::frame::{FrameControl, SequenceControl},
};

/// Represents an IEEE 802.11 Beacon frame header.
/// Management frames like Beacon typically have 3 addresses.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone)]
pub struct BeaconFrameHeader {
    /// Frame Control field. Type=MGMT, Subtype=BEACON.
    pub frame_control: FrameControl,
    /// Duration field.
    pub duration: U16<LittleEndian>,
    /// Address 1: Destination MAC Address (typically broadcast
    /// FF:FF:FF:FF:FF:FF).
    pub da: MacAddr,
    /// Address 2: Source MAC Address (Transmitter Address / BSSID).
    pub sa: MacAddr,
    /// Address 3: BSSID.
    pub bssid: MacAddr,
    /// Sequence Control field.
    pub sequence_control: SequenceControl,
    // Followed by fixed parameters (Timestamp, Beacon Interval, Capability Info)
    // and then tagged parameters (SSID, Rates, etc.).
}

/// Fixed parameters in a Beacon or Probe Response frame (12 bytes).
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone)]
pub struct BeaconFixedFields {
    /// Timestamp (8 bytes).
    pub timestamp: [u8; 8],
    /// Beacon Interval (2 bytes).
    pub beacon_interval: U16<LittleEndian>,
    /// Capability Information (2 bytes).
    pub capabilities: U16<LittleEndian>,
}
