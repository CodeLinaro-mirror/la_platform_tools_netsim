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

/// Standard UICC File Identifiers (ISO/IEC 7816-4 §5.3 / 3GPP TS 51.011 / TS
/// 31.102).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum UiccFileId {
    // Master File & Application Dedicated Files
    MasterFile = 0x3F00,
    AdfDefault = 0x7FFF,

    // Dedicated Files
    Telecom = 0x7F10,

    // Elementary Files
    Imsi = 0x6F07,
    Iccid = 0x2FE2,
    ForbiddenPlmn = 0x6F7B,
    Msisdn = 0x6F40,
    MailboxDialingNumbers = 0x6FC7,
    AdministrativeData = 0x6FAD,
    FixedDialingNumbers = 0x6F3B,
}

impl UiccFileId {
    pub const fn as_u16(self) -> u16 {
        self as u16
    }

    pub const fn is_virtual_fallback(self) -> bool {
        matches!(
            self,
            Self::Iccid
                | Self::Imsi
                | Self::Msisdn
                | Self::ForbiddenPlmn
                | Self::MailboxDialingNumbers
                | Self::AdministrativeData
        )
    }

    pub fn is_virtual_fallback_id(file_id: u16) -> bool {
        Self::try_from(file_id).is_ok_and(Self::is_virtual_fallback)
    }
}

impl From<UiccFileId> for u16 {
    fn from(id: UiccFileId) -> Self {
        id as u16
    }
}

impl TryFrom<u16> for UiccFileId {
    type Error = ();

    fn try_from(val: u16) -> Result<Self, Self::Error> {
        match val {
            0x3F00 => Ok(Self::MasterFile),
            0x7FFF => Ok(Self::AdfDefault),
            0x7F10 => Ok(Self::Telecom),
            0x6F07 => Ok(Self::Imsi),
            0x2FE2 => Ok(Self::Iccid),
            0x6F7B => Ok(Self::ForbiddenPlmn),
            0x6F40 => Ok(Self::Msisdn),
            0x6FC7 => Ok(Self::MailboxDialingNumbers),
            0x6FAD => Ok(Self::AdministrativeData),
            0x6F3B => Ok(Self::FixedDialingNumbers),
            _ => Err(()),
        }
    }
}

// Abbreviated Dialing Numbers (ADN) Record Layout (TS 31.102 §4.4.2.3 / TS
// 51.011 §10.5.1)
pub const ADN_ALPHA_IDENTIFIER_LEN: usize = 14;
pub const ADN_DIALING_NUMBER_LEN: usize = 10;
pub const ADN_CAPABILITY_EXT_BYTES: [u8; 2] = [0xFF, 0xFF];

// Elementary File Record Lengths (Linear Fixed)
pub const EF_MSISDN_RECORD_LEN: usize = 28;
pub const EF_MBDN_RECORD_LEN: usize = 38;
pub const EF_FDN_RECORD_LEN: usize = 28;

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

// GET_RESPONSE (0xC0) status words
pub const SW_BYTES_REMAINING_PREFIX: u8 = 0x61;
