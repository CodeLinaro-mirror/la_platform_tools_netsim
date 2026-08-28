// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/constants.rs

use std::time::Duration;

use crate::types::CtecTechnology;

pub const CALL_RING_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_PLMN: &str = "310260";
pub const DEFAULT_OPERATOR_NAME_LONG: &str = "Android Virtual Operator";
pub const DEFAULT_OPERATOR_NAME_SHORT: &str = "Android";
pub const DEFAULT_MSISDN_PREFIX: &str = "15555211";
pub const DEFAULT_FALLBACK_MSISDN: &str = "15551234567";

/// Standard AT signal strength unknown value.
pub const CSQ_SIGNAL_UNKNOWN: u8 = 99;

/// List of CTEC technologies supported by this simulator.
pub const SUPPORTED_CTEC_TECHS: &[CtecTechnology] =
    &[CtecTechnology::Gsm, CtecTechnology::Wcdma, CtecTechnology::Lte, CtecTechnology::Nr];

pub const MF_FILE_ID: u16 = 0x3F00;
pub const ADF_DEFAULT_FILE_ID: u16 = 0x7FFF;

pub const TAG_DF_NAME: u8 = 0x84;

// ISO 7816-4 APDU Status Words (SW)
pub const SW_SUCCESS: u16 = 0x9000;
pub const SW_WRONG_LENGTH: u16 = 0x6700;
pub const SW_FILE_NOT_FOUND: u16 = 0x6A82;
pub const SW_INCORRECT_PARAMS: u16 = 0x6A86;
pub const SW_REFERENCED_DATA_NOT_FOUND: u16 = 0x6A88;
pub const SW_CLASS_NOT_SUPPORTED: u16 = 0x6E00;
pub const SW_INS_NOT_SUPPORTED: u16 = 0x6D00;
pub const SW_TECHNICAL_PROBLEM: u16 = 0x6F00;
pub const SW_NO_CHANNEL_AVAILABLE: u16 = 0x6A81;
#[allow(dead_code)]
pub const SW_INCOMPATIBLE_FILE_STRUCTURE: u16 = 0x6981;
#[allow(dead_code)]
pub const SW_COMMAND_NOT_ALLOWED: u16 = 0x6986;

// GET_RESPONSE (0xC0) status words
pub const SW_BYTES_REMAINING_PREFIX: u8 = 0x61;
