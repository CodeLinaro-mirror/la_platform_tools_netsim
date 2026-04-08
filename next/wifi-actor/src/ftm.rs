// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use netsim_model::device::Position;
use netsim_packets::ieee80211::{
    action::{
        category,
        public_action,
        // FtmRequest, // In action module
    },
    Ieee80211,
};
use tracing::{debug, warn};
use zerocopy::IntoBytes;

const SPEED_OF_LIGHT: f64 = 299_792_458.0; // m/s
const PICOSECONDS_PER_SECOND: u64 = 1_000_000_000_000;

/// Handles an FTM Request packet and generates responses.
///
/// If the packet is a valid FTM Request, this function calculates the
/// distance-based timestamps (Time of Flight) and returns a list of response
/// frames (FTM Initial + FTM Measurement).
pub fn handle_ftm_request(
    packet: &Ieee80211,
    initiator_pos: &Position,
    responder_pos: &Position,
) -> Option<Vec<Bytes>> {
    // 1. Verify Packet is Public Action -> FTM Request
    // We assume the caller checked the basic frame type, but we verify Action
    // content.
    let (category_val, action_val, trigger) = parse_ftm_request(packet)?;

    if category_val != category::PUBLIC || action_val != public_action::FTM_REQUEST {
        return None;
    }

    debug!("FTM Request received. Trigger: {}", trigger);

    // 2. Calculate Distance & RTT
    let distance = calculate_distance(initiator_pos, responder_pos);
    if distance <= 0.0 {
        warn!("FTM distance is zero or invalid, using minimal RTT");
    }

    // Time of Flight (seconds) = distance / c
    let tof_seconds = distance / SPEED_OF_LIGHT;
    // RTT = 2 * ToF
    let rtt_picoseconds = (tof_seconds * 2.0 * PICOSECONDS_PER_SECOND as f64) as u64;

    // Simulate Timestamps
    // t1: Time of Departure (Responder) - arbitrary base (0)
    // t4: Time of Arrival (Responder) - t1 + RTT
    // Timestamps are 48-bit (6 bytes).

    let t1_bytes = 0u64.to_le_bytes(); // Start at 0
    let t4_bytes = rtt_picoseconds.to_le_bytes(); // RTT later

    // 3. Construct Responses
    let mut responses = Vec::new();

    // Source/Dest are swapped for response
    // Initator (Source of Request) -> Destination of Response
    // Responder (Dest of Request) -> Source of Response
    let dest = packet.get_structure_addr2(); // SA of request
    let src = packet.get_structure_addr1(); // DA of request (us)
    let bssid = packet.get_structure_addr3();

    // Helper to build Action Frame
    let build_action = |body: &[u8]| -> Bytes {
        let mut frame = Vec::new();
        // 802.11 Header (Simplified Management Action)
        // FC: Action (0xD0), Flags...
        let fc = netsim_packets::ieee80211::FrameControl::new(0x00D0);
        let header = netsim_packets::ieee80211::MacHeader3Addr {
            frame_control: fc,
            duration_id: zerocopy::U16::new(0),
            addr1: dest,
            addr2: src,
            addr3: bssid,
            sequence_control: netsim_packets::ieee80211::SequenceControl::new(0),
        };
        frame.extend_from_slice(header.as_bytes());
        frame.extend_from_slice(body);
        Bytes::from(frame)
    };

    // Frame 1: FTM Action (Initial)
    let mut body1 = Vec::new();
    body1.push(category::PUBLIC);
    body1.push(public_action::FINE_TIMING_MEASUREMENT);
    body1.push(0); // Dialog Token
    body1.push(0); // Follow Up Dialog Token
                   // Zero timestamps
    body1.extend_from_slice(&[0u8; 6]); // TOD
    body1.extend_from_slice(&[0u8; 6]); // TOA
    body1.extend_from_slice(&[0u8; 6]); // TOD Error / etc
                                        // body1.extend_from_slice(&[0u8; 6]); // TOA Error

    responses.push(build_action(&body1));

    // Frame 2: FTM Action (With Timestamps)
    let mut body2 = Vec::new();
    body2.push(category::PUBLIC);
    body2.push(public_action::FINE_TIMING_MEASUREMENT);
    body2.push(0); // Dialog Token
    body2.push(0); // Follow Up Dialog Token
                   // Timestamps (48-bit usually)
    body2.extend_from_slice(&t1_bytes[0..6]); // TOD
    body2.extend_from_slice(&t4_bytes[0..6]); // TOA (using t4 = RTT for simplicity, implying t1=0)
    body2.extend_from_slice(&[0u8; 6]); // Error

    responses.push(build_action(&body2));

    Some(responses)
}

fn parse_ftm_request(packet: &Ieee80211) -> Option<(u8, u8, u8)> {
    // Basic checks provided by caller, just parse body
    // Body starts at offset 24 for Mgmt frames (Header=24 bytes)
    // Check if it's Action frame
    if packet.stype() != netsim_packets::ieee80211::management_subtype::ACTION {
        return None;
    }

    let bytes = packet.as_bytes();
    if bytes.len() < 24 + 3 {
        return None;
    }

    let category = bytes[24];
    let action = bytes[25];
    let trigger = bytes[26];

    Some((category, action, trigger))
}

fn calculate_distance(p1: &Position, p2: &Position) -> f64 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    let dz = p1.z - p2.z;
    (dx * dx + dy * dy + dz * dz).sqrt() as f64
}

// Helpers for extracting addresses directly if inherent helpers are restrictive
trait AddrHelpers {
    fn get_structure_addr1(&self) -> netsim_packets::ieee80211::MacAddress;
    fn get_structure_addr2(&self) -> netsim_packets::ieee80211::MacAddress;
    fn get_structure_addr3(&self) -> netsim_packets::ieee80211::MacAddress;
}

impl AddrHelpers for Ieee80211 {
    fn get_structure_addr1(&self) -> netsim_packets::ieee80211::MacAddress {
        self.get_addr1()
    }
    fn get_structure_addr2(&self) -> netsim_packets::ieee80211::MacAddress {
        self.get_addr2()
    }
    fn get_structure_addr3(&self) -> netsim_packets::ieee80211::MacAddress {
        self.get_addr3()
    }
}
