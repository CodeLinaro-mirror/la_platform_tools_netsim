// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

pub struct UciCodec;

impl Decoder for UciCodec {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        const UCI_HEADER_SIZE: usize = 4;
        const UCI_MT_MASK: u8 = 0xE0;
        const UCI_MT_SHIFT: u8 = 5;
        const UCI_MT_DATA: u8 = 0;

        if src.len() < UCI_HEADER_SIZE {
            return Ok(None);
        }

        let mt = (src[0] & UCI_MT_MASK) >> UCI_MT_SHIFT;

        let payload_len = if mt == UCI_MT_DATA {
            // Data Packet: 2-byte payload length at index 2 and 3
            u16::from_le_bytes([src[2], src[3]]) as usize
        } else {
            // Control Packet: 1-byte payload length at index 3
            src[3] as usize
        };

        let total_len = UCI_HEADER_SIZE + payload_len;

        if src.len() < total_len {
            src.reserve(total_len - src.len());
            return Ok(None);
        }

        let data = src.split_to(total_len);
        Ok(Some(data.freeze()))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BufMut;

    use super::*;

    #[test]
    fn test_decode_uci_control_packet() {
        let mut codec = UciCodec;
        let mut buf = BytesMut::new();
        buf.put_slice(&[0x21, 0x01, 0x00, 0x02, 0xaa, 0xbb]);

        let packet = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(packet.len(), 6);
        assert_eq!(packet.as_ref(), &[0x21, 0x01, 0x00, 0x02, 0xaa, 0xbb]);
    }

    #[test]
    fn test_decode_uci_data_packet() {
        let mut codec = UciCodec;
        let mut buf = BytesMut::new();
        buf.put_slice(&[0x00, 0x01, 0x02, 0x00, 0xcc, 0xdd]);

        let packet = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(packet.len(), 6);
        assert_eq!(packet.as_ref(), &[0x00, 0x01, 0x02, 0x00, 0xcc, 0xdd]);
    }

    #[test]
    fn test_decode_uci_partial() {
        let mut codec = UciCodec;
        let mut buf = BytesMut::new();
        buf.put_slice(&[0x00, 0x01, 0x02, 0x00]);

        let res = codec.decode(&mut buf).unwrap();
        assert!(res.is_none());
        assert_eq!(buf.len(), 4);

        buf.put_slice(&[0xcc, 0xdd]);
        let packet = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(packet.len(), 6);
        assert_eq!(packet.as_ref(), &[0x00, 0x01, 0x02, 0x00, 0xcc, 0xdd]);
    }
}
