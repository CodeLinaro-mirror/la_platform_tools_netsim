// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/constants.rs

use std::time::Duration;

pub const CALL_RING_TIMEOUT: Duration = Duration::from_secs(1);
pub const GPRS_DIAL_PREFIX: &[u8] = b"*99";
pub const DEFAULT_PLMN: &str = "310260";
pub const DEFAULT_OPERATOR_NAME_LONG: &str = "Android Virtual Operator";
pub const DEFAULT_OPERATOR_NAME_SHORT: &str = "Android";

/// Returns true if the dial string is a standard GPRS dialing command.
/// Standard GPRS dialing formats include *99#, *99*<cid>#, or *99***<cid>#.
pub fn is_gprs_dial(number: &[u8]) -> bool {
    number.starts_with(GPRS_DIAL_PREFIX)
        && number.ends_with(b"#")
        && number.get(3).is_some_and(|&c| c == b'*' || c == b'#')
}

/// Standard AT signal strength unknown value.
pub const CSQ_SIGNAL_UNKNOWN: u8 = 99;

/// Facility code for SIM PIN lock (`AT+CLCK="SC"` / `AT+CPWD="SC"`).
pub const FACILITY_SIM_PIN: &str = "SC";

/// Network Technology indices.
pub mod modem_tech_index {
    pub const GSM: u8 = 0;
    pub const WCDMA: u8 = 1;
    pub const LTE: u8 = 5;
    pub const NR: u8 = 6;
}

/// Network Technology bitmask values.
#[allow(dead_code)]
pub mod modem_tech {
    use super::modem_tech_index;
    pub const GSM: u8 = 1 << modem_tech_index::GSM;
    pub const WCDMA: u8 = 1 << modem_tech_index::WCDMA;
    pub const LTE: u8 = 1 << modem_tech_index::LTE;
    pub const NR: u8 = 1 << modem_tech_index::NR;
}

/// Default CTEC current network technology (LTE).
pub const CTEC_DEFAULT_CURRENT_TECH: u8 = modem_tech::LTE;
/// Default CTEC preferred network technology mask (NR).
pub const CTEC_DEFAULT_PREFERRED_MASK: u32 = modem_tech::NR as u32;

/// List of CTEC technologies supported by this simulator.
pub const SUPPORTED_CTEC_INDEXES: &[u8] =
    &[modem_tech_index::GSM, modem_tech_index::WCDMA, modem_tech_index::LTE, modem_tech_index::NR];

/// Access Technology (AcT) values as defined in 3GPP TS 27.007 (e.g. +COPS,
/// +CREG)
pub mod access_technology {
    pub const GSM: u8 = 0;
    pub const WCDMA: u8 = 2;
    pub const LTE: u8 = 7;
    pub const NR: u8 = 11;
}
