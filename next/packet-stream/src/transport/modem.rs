// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::{Buf, Bytes, BytesMut};
use tokio_util::codec::Decoder;

pub struct ModemCodec;

impl Decoder for ModemCodec {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if src.is_empty() {
                return Ok(None);
            }

            // AT commands are terminated by \r or \n.
            // Interactive prompts (like SMS input) terminate on Ctrl+Z (\x1a) or ESC
            // (\x1b).
            if let Some(pos) =
                src.iter().position(|&b| b == b'\n' || b == b'\r' || b == b'\x1a' || b == b'\x1b')
            {
                let delimiter = src[pos];
                if delimiter == b'\n' || delimiter == b'\r' {
                    let data = src.split_to(pos);
                    src.advance(1); // Consume the delimiter
                    if delimiter == b'\r' && !src.is_empty() && src[0] == b'\n' {
                        src.advance(1); // Consume trailing \n
                    }
                    if data.is_empty() {
                        continue;
                    }
                    return Ok(Some(data.freeze()));
                } else {
                    // Include \x1a or \x1b in the yielded frame for parser evaluation.
                    let data = src.split_to(pos + 1);
                    return Ok(Some(data.freeze()));
                }
            } else {
                if src.len() >= 4096 {
                    let data = src.split_to(4096);
                    return Ok(Some(data.freeze()));
                }
                return Ok(None);
            }
        }
    }
}
