// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// src/pdu.rs

// Represents the fixed-size header of a Command APDU (ISO/IEC 7816-4).
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub struct CommandApdu<'a> {
    pub cla: u8,
    pub ins: u8,
    pub p1: u8,
    pub p2: u8,
    pub p3: u8, // Represents Lc or Le
    pub data: &'a [u8],
}
