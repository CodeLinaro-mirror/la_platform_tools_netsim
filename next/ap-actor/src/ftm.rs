// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_packets::{
    FineTimingMeasurement, FrameControl, MacAddr, MacHeader3Addr, category, public_action,
};
use zerocopy::{IntoBytes, U16};

use crate::ap_actor::ApState;

/// FTM Responder Logic
#[derive(Debug)]
pub(crate) struct FtmResponder {
    // Stateless responder: responds immediately to each request.
}

impl FtmResponder {
    /// Handle an incoming FTM Request and generate the necessary response
    /// frames.
    ///
    /// Handle an incoming FTM Request and generate the necessary response
    /// frames.
    ///
    /// Implements a simplified Single-Burst FTM exchange (ASAP=1):
    /// 1. Receive FTM Request (Trigger=1).
    /// 2. Send Initial FTM Frame (Dialog Token=N, Follow Up=0) with t1/t4
    ///    placeholders.
    /// 3. Send Follow-Up FTM Frame (Dialog Token=N, Follow Up=N) containing the
    ///    simulated timestamps (t1, t4) calculated based on a fixed distance.
    ///
    /// Note: In a real physical exchange, t4 would be captured upon packet
    /// arrival. Here, we pre-calculate timestamps to simulate a specific
    /// distance (RTT).
    pub fn handle_ftm_request(ap: &mut ApState, src: MacAddr, dialog_token: u8) -> Vec<Vec<u8>> {
        let mut responses = Vec::new();

        // 1. Initial FTM Frame (ASAP=1)
        // Indicates measurement start. Contains dummy timestamps (0).
        let ftm_body_1 = FineTimingMeasurement {
            dialog_token,
            follow_up_dialog_token: 0,
            tod: [0; 6],
            toa: [0; 6],
            tod_error: [0; 6], // Simplified padding
        };

        // Build Action Frame
        let header = MacHeader3Addr {
            frame_control: FrameControl::new(0x00D0), // Action
            duration_id: U16::new(0),
            addr1: src,
            addr2: ap.config.bssid,
            addr3: ap.config.bssid,
            sequence_control: ap.next_seq_control(),
        };

        let mut frame_1 = Vec::new();
        frame_1.extend_from_slice(header.as_bytes());
        frame_1.push(category::PUBLIC);
        frame_1.push(public_action::FINE_TIMING_MEASUREMENT);
        frame_1.extend_from_slice(ftm_body_1.as_bytes());

        responses.push(frame_1);

        // 2. Second FTM Frame (Follow Up) containing the "measurement" from Frame 1.
        // We simulate the RTT by calculating t4 based on a fixed flight time (distance)
        // plus a fixed SIFS processing time.
        //
        // RTT = (t4 - t1) - (t3 - t2)
        // We report t1 and t4. The client (Station) will measure t2 and t3 locally.

        let t1_val: u64 = 1000000000000; // Arbitrary Start: 1000s in ps
        // TODO: Calculate distance based on src/dest positions if available.
        // For now, default to 5.0 meters.
        let dist_m = 5.0;

        // Speed of light approx 0.3m/ns
        let flight_ns = (dist_m / 0.3) as u64;
        let flight_ps = flight_ns * 1000;
        let sifs_ps = 10000 * 1000; // 10us SIFS

        // t4 = t1 + Flight + SIFS + Flight
        // Note: This presumes the client turnaround is exactly SIFS.
        let t4_val = t1_val + (2 * flight_ps) + sifs_ps;

        // Encode 48-bit timestamps
        let t1_bytes = &t1_val.to_le_bytes()[0..6];
        let t4_bytes = &t4_val.to_le_bytes()[0..6];

        let ftm_body_2 = FineTimingMeasurement {
            dialog_token,
            follow_up_dialog_token: dialog_token, // Reference to previous?
            tod: t1_bytes.try_into().unwrap(),
            toa: t4_bytes.try_into().unwrap(),
            tod_error: [0; 6],
        };

        let mut frame_2 = Vec::new();
        frame_2.extend_from_slice(header.as_bytes());
        frame_2.push(category::PUBLIC);
        frame_2.push(public_action::FINE_TIMING_MEASUREMENT);
        frame_2.extend_from_slice(ftm_body_2.as_bytes());

        responses.push(frame_2);

        responses
    }
}
