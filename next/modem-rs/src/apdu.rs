// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! APDU parsing helpers for ISO/IEC 7816-4.

use crate::constants::SW_WRONG_LENGTH;

/// Distinguishes the four standard APDU command-response cases defined in
/// ISO/IEC 7816-4 § 5.1.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ApduCase<'a> {
    /// Case 1 (ISO/IEC 7816-4 § 5.1): 4-byte header only. No command data sent,
    /// no response data expected.
    CommandOnly,
    /// Case 2 (ISO/IEC 7816-4 § 5.1): Header + Le. No command data sent,
    /// response data of length `expected_length` expected.
    ResponseExpected { expected_length: u8 },
    /// Case 3 (ISO/IEC 7816-4 § 5.1): Header + Lc + Data. Command data payload
    /// sent, no response data expected.
    DataOnly { data: &'a [u8] },
    /// Case 4 (ISO/IEC 7816-4 § 5.1): Header + Lc + Data + Le. Command data
    /// sent and response data expected.
    DataAndResponse { data: &'a [u8], expected_length: u8 },
}
/// APDU Instructions as defined in ISO/IEC 7816-4 & ETSI TS 102 221.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    ReadBinary,
    ReadRecord,
    UpdateRecord,
    Select,
    UpdateBinary,
    Status,
    ManageChannel,
    GetResponse,
    Unknown(u8),
}

impl From<u8> for Instruction {
    fn from(val: u8) -> Self {
        match val {
            0xB0 => Self::ReadBinary,
            0xB2 => Self::ReadRecord,
            0xDC => Self::UpdateRecord,
            0xA4 => Self::Select,
            0xD6 => Self::UpdateBinary,
            0xF2 => Self::Status,
            0x70 => Self::ManageChannel,
            0xC0 => Self::GetResponse,
            other => Self::Unknown(other),
        }
    }
}

/// Parsed APDU components according to ISO/IEC 7816-4.
#[derive(Debug, PartialEq, Clone)]
pub struct ParsedApdu<'a> {
    pub class: u8,
    pub ins: Instruction,
    pub p1: u8,
    pub p2: u8,
    pub case: ApduCase<'a>,
}

impl<'a> ParsedApdu<'a> {
    /// Returns the APDU command data payload if present (`DataOnly` or
    /// `DataAndResponse`), or an empty slice.
    pub fn data(&self) -> &'a [u8] {
        match self.case {
            ApduCase::DataOnly { data } | ApduCase::DataAndResponse { data, .. } => data,
            _ => &[],
        }
    }

    /// Returns the expected response length `Le` if present (`ResponseExpected`
    /// or `DataAndResponse`).
    pub fn expected_length(&self) -> Option<u8> {
        match self.case {
            ApduCase::ResponseExpected { expected_length }
            | ApduCase::DataAndResponse { expected_length, .. } => Some(expected_length),
            _ => None,
        }
    }

    /// Parses raw APDU bytes into structured [`ParsedApdu`] components.
    ///
    /// Supports short length APDUs as defined in ISO/IEC 7816-4:
    /// - Case 1: CLA INS P1 P2 (4 bytes)
    /// - Case 2: CLA INS P1 P2 Le (5 bytes)
    /// - Case 3: CLA INS P1 P2 Lc Data (5 + Lc bytes)
    /// - Case 4: CLA INS P1 P2 Lc Data Le (6 + Lc bytes)
    ///
    /// Returns `SW_WRONG_LENGTH` (0x6700) if the structure is invalid.
    pub fn parse(apdu_bytes: &'a [u8]) -> Result<Self, u16> {
        let (cla, ins, p1, p2, case) = match *apdu_bytes {
            [cla, ins, p1, p2] => (cla, ins, p1, p2, ApduCase::CommandOnly),
            [cla, ins, p1, p2, expected_length] => {
                (cla, ins, p1, p2, ApduCase::ResponseExpected { expected_length })
            }
            [cla, ins, p1, p2, lc, ref rest @ ..] => {
                let len = lc as usize;
                if rest.len() == len {
                    (cla, ins, p1, p2, ApduCase::DataOnly { data: rest })
                } else if rest.len() == len + 1 {
                    (
                        cla,
                        ins,
                        p1,
                        p2,
                        ApduCase::DataAndResponse {
                            data: &rest[..len],
                            expected_length: rest[len],
                        },
                    )
                } else {
                    return Err(SW_WRONG_LENGTH);
                }
            }
            _ => return Err(SW_WRONG_LENGTH),
        };
        Ok(Self { class: cla, ins: Instruction::from(ins), p1, p2, case })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apdu_cases() {
        // Case 1
        let parsed = ParsedApdu::parse(&[0x00, 0xA4, 0x00, 0x04]).unwrap();
        assert_eq!(
            parsed,
            ParsedApdu {
                class: 0x00,
                ins: Instruction::Select,
                p1: 0x00,
                p2: 0x04,
                case: ApduCase::CommandOnly
            }
        );
        assert!(parsed.data().is_empty());
        assert_eq!(parsed.expected_length(), None);

        // Case 2
        let parsed = ParsedApdu::parse(&[0x00, 0xC0, 0x00, 0x00, 0x05]).unwrap();
        assert_eq!(
            parsed,
            ParsedApdu {
                class: 0x00,
                ins: Instruction::GetResponse,
                p1: 0x00,
                p2: 0x00,
                case: ApduCase::ResponseExpected { expected_length: 5 }
            }
        );
        assert!(parsed.data().is_empty());
        assert_eq!(parsed.expected_length(), Some(5));

        // Case 3
        let parsed = ParsedApdu::parse(&[0x00, 0xD6, 0x00, 0x00, 0x02, 0x11, 0x22]).unwrap();
        assert_eq!(
            parsed,
            ParsedApdu {
                class: 0x00,
                ins: Instruction::UpdateBinary,
                p1: 0x00,
                p2: 0x00,
                case: ApduCase::DataOnly { data: &[0x11, 0x22] }
            }
        );
        assert_eq!(parsed.data(), &[0x11, 0x22]);
        assert_eq!(parsed.expected_length(), None);

        // Case 4 with Lc=0 (empty data) and Le present
        let parsed = ParsedApdu::parse(&[0x00, 0xA4, 0x00, 0x04, 0x00, 0x05]).unwrap();
        assert_eq!(
            parsed,
            ParsedApdu {
                class: 0x00,
                ins: Instruction::Select,
                p1: 0x00,
                p2: 0x04,
                case: ApduCase::DataAndResponse { data: &[], expected_length: 5 }
            }
        );
        assert!(parsed.data().is_empty());
        assert_eq!(parsed.expected_length(), Some(5));

        // Invalid length
        assert_eq!(ParsedApdu::parse(&[0x00, 0xA4, 0x00]), Err(SW_WRONG_LENGTH));
        assert_eq!(ParsedApdu::parse(&[0x00, 0xD6, 0x00, 0x00, 0x02, 0x11]), Err(SW_WRONG_LENGTH));
        assert_eq!(ParsedApdu::parse(&[]), Err(SW_WRONG_LENGTH));
        assert_eq!(ParsedApdu::parse(&[0x00]), Err(SW_WRONG_LENGTH));
    }
}
