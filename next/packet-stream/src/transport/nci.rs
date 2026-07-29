// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

pub struct NciCodec;

impl Decoder for NciCodec {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        const NCI_HEADER_SIZE: usize = 3;

        if src.len() < NCI_HEADER_SIZE {
            return Ok(None);
        }

        // NCI packets always have the payload length at index 2.
        let payload_len = src[2] as usize;
        // Max payload length is 255, so max total_len is 258.
        let total_len = NCI_HEADER_SIZE + payload_len;

        if src.len() < total_len {
            src.reserve(total_len - src.len());
            return Ok(None);
        }

        let data = src.split_to(total_len);
        Ok(Some(data.freeze()))
    }
}
