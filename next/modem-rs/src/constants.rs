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

/// Default LTE RSSI value (99 represents unknown/unavailable).
pub const CSQ_LTE_RSSI_DEFAULT: u8 = 99;
/// Default LTE RSRP value (44 represents typical strong signal).
pub const CSQ_LTE_RSRP_DEFAULT: u8 = 44;
