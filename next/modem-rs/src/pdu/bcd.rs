// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! BCD (Binary Coded Decimal) helper functions for SMS PDU encoding/decoding.

/// Decodes semi-octet (BCD) representation to a string.
/// Swaps every pair of nibbles. Stops on padding 'F' (0x0F).
/// Supports GSM special characters (*, #, a, b, c) mapped to 10..14.
pub fn bcd_to_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let low = b & 0x0F;
        let high = b >> 4;

        match low {
            0..=9 => s.push((b'0' + low) as char),
            10 => s.push('*'),
            11 => s.push('#'),
            12 => s.push('a'),
            13 => s.push('b'),
            14 => s.push('c'),
            0x0F => break,
            _ => unreachable!(),
        }

        match high {
            0..=9 => s.push((b'0' + high) as char),
            10 => s.push('*'),
            11 => s.push('#'),
            12 => s.push('a'),
            13 => s.push('b'),
            14 => s.push('c'),
            0x0F => break,
            _ => unreachable!(),
        }
    }
    s
}

/// Encodes a string of digits and special characters (*, #, a, b, c)
/// to semi-octet (BCD) representation.
/// Swaps every pair of digits. Pads with 'F' (0x0F) if odd length.
#[cfg(test)]
pub fn string_to_bcd(s: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((s.len() + 1) / 2);
    let mut bytes_iter = s.bytes();

    let char_to_bcd = |b: u8| -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'*' => 10,
            b'#' => 11,
            b'a' => 12,
            b'b' => 13,
            b'c' => 14,
            _ => 0,
        }
    };

    while let Some(b1) = bytes_iter.next() {
        let low = char_to_bcd(b1);
        let high = match bytes_iter.next() {
            Some(b2) => char_to_bcd(b2),
            None => 0x0F, // Pad
        };
        bytes.push((high << 4) | low);
    }
    bytes
}

/// Converts a byte to its BCD representation (swapped nibbles).
/// Assumes value is less than 100.
pub fn to_bcd_byte(val: u8) -> u8 {
    debug_assert!(val < 100, "Value must be less than 100 for BCD encoding");
    let val = val % 100; // Safety net for release mode (prevent bit-clobbering)
    let hi = val / 10;
    let lo = val % 10;
    (lo << 4) | hi
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcd_to_string() {
        let bytes = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(bcd_to_string(&bytes), "21436587");

        let bytes_padded = [0x12, 0x34, 0xF5];
        assert_eq!(bcd_to_string(&bytes_padded), "21435");
    }

    #[test]
    fn test_string_to_bcd() {
        assert_eq!(string_to_bcd("21436587"), vec![0x12, 0x34, 0x56, 0x78]);
        assert_eq!(string_to_bcd("21435"), vec![0x12, 0x34, 0xF5]);
    }

    #[test]
    fn test_bcd_special_characters() {
        let s = "*123#abc";
        let bcd = string_to_bcd(s);
        // * -> 10 (A), 1 -> 1. Byte 1: (1 << 4) | 10 = 0x1A
        // 2 -> 2, 3 -> 3. Byte 2: (3 << 4) | 2 = 0x32
        // # -> 11 (B), a -> 12 (C). Byte 3: (12 << 4) | 11 = 0xCB
        // b -> 13 (D), c -> 14 (E). Byte 4: (14 << 4) | 13 = 0xED
        assert_eq!(bcd, vec![0x1A, 0x32, 0xCB, 0xED]);
        assert_eq!(bcd_to_string(&bcd), s);
    }

    #[test]
    fn test_to_bcd_byte() {
        assert_eq!(to_bcd_byte(0), 0x00);
        assert_eq!(to_bcd_byte(45), 0x54);
        assert_eq!(to_bcd_byte(99), 0x99);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Value must be less than 100")]
    fn test_to_bcd_byte_panic_in_debug() {
        super::to_bcd_byte(100);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn test_to_bcd_byte_release_safety() {
        // Verifies the modulo safety net (105 % 100 = 5 -> BCD 05 -> swapped 0x50)
        assert_eq!(to_bcd_byte(105), 0x50);
    }
}
