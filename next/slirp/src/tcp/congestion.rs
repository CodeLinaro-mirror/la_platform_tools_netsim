// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! TCP congestion control implementation.

pub(crate) const INITIAL_CWND_PACKETS: u32 = 10;
pub(crate) const INITIAL_SSTHRESH: u32 = 65535;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CongestionControl {
    pub cwnd: u32,
    pub ssthresh: u32,
}

impl CongestionControl {
    pub fn new(mss: u16) -> Self {
        Self { cwnd: INITIAL_CWND_PACKETS * mss as u32, ssthresh: INITIAL_SSTHRESH }
    }

    pub fn on_ack(&mut self, mss: u16) {
        if self.cwnd < self.ssthresh {
            // Slow start
            self.cwnd += mss as u32;
        } else {
            // Congestion avoidance
            self.cwnd += (mss as u32 * mss as u32) / self.cwnd;
        }
    }

    pub fn on_retransmission(&mut self, mss: u16) {
        self.ssthresh = self.cwnd / 2;
        self.cwnd = mss as u32;
    }
}
