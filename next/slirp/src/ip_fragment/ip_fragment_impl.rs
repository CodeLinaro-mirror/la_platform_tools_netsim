// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! IP fragmentation and reassembly.

use std::collections::HashMap;

use netsim_packets::Ipv4Header;
use zerocopy::{FromBytes, IntoBytes};

pub const MTU: usize = 1500;

pub fn fragment(packet: &[u8]) -> Vec<Vec<u8>> {
    let (ipv4_header, payload) = Ipv4Header::parse(packet).unwrap();
    let ipv4_header_len = ipv4_header.header_length();
    let max_payload_len = (MTU - ipv4_header_len) & !7;

    let mut fragments = Vec::new();
    let mut offset = 0;

    while offset < payload.len() {
        let fragment_len = std::cmp::min(max_payload_len, payload.len() - offset);
        let mut fragment_packet = vec![0u8; ipv4_header_len + fragment_len];
        let (fragment_ipv4_header_slice, fragment_payload) =
            fragment_packet.split_at_mut(ipv4_header_len);
        let fragment_ipv4_header = Ipv4Header::mut_from_bytes(fragment_ipv4_header_slice).unwrap();

        fragment_ipv4_header.as_mut_bytes().copy_from_slice(ipv4_header.as_bytes());
        fragment_payload.copy_from_slice(&payload[offset..offset + fragment_len]);

        let mut flags_fragment_offset = ipv4_header.flags_fragment_offset.get();
        if offset + fragment_len < payload.len() {
            flags_fragment_offset |= 0x2000; // More fragments
        }
        flags_fragment_offset |= (offset / 8) as u16;
        fragment_ipv4_header.flags_fragment_offset.set(flags_fragment_offset);

        fragment_ipv4_header.total_length.set((ipv4_header_len + fragment_len) as u16);
        fragment_ipv4_header.header_checksum.set(0);
        // The checksum will be calculated by the caller.

        fragments.push(fragment_packet);
        offset += fragment_len;
    }

    fragments
}

struct FragmentCache {
    fragments: Vec<Vec<u8>>,
    total_len: usize,
    last_fragment_received: bool,
}

impl FragmentCache {
    fn new() -> Self {
        Self { fragments: Vec::new(), total_len: 0, last_fragment_received: false }
    }

    fn is_complete(&self) -> bool {
        if !self.last_fragment_received {
            return false;
        }

        let mut fragments = self.fragments.clone();
        fragments.sort_by_key(|a| {
            let (h, _) = Ipv4Header::parse(a).unwrap();
            h.flags_fragment_offset.get() & 0x1FFF
        });

        let mut current_offset = 0;
        for fragment in &fragments {
            let (header, payload) = Ipv4Header::parse(fragment).unwrap();
            let offset = (header.flags_fragment_offset.get() & 0x1FFF) * 8;
            if offset as usize != current_offset {
                return false;
            }
            current_offset += payload.len();
        }
        if current_offset != self.total_len {
            return false;
        }
        true
    }
}

#[derive(Default)]
pub struct Reassembler {
    cache: HashMap<u16, FragmentCache>,
}

impl Reassembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reassemble(&mut self, fragment: &[u8]) -> Option<Vec<u8>> {
        let (ipv4_header, payload) = Ipv4Header::parse(fragment).unwrap();
        let identification = ipv4_header.identification.get();
        let more_fragments = ipv4_header.flags_fragment_offset.get() & 0x2000 != 0;

        let cache_entry = self.cache.entry(identification).or_insert_with(FragmentCache::new);
        cache_entry.fragments.push(fragment.to_vec());

        if !more_fragments {
            let offset = (ipv4_header.flags_fragment_offset.get() & 0x1FFF) * 8;
            cache_entry.total_len = offset as usize + payload.len();
            cache_entry.last_fragment_received = true;
        }

        if cache_entry.is_complete() {
            let cache_entry = self.cache.remove(&identification).unwrap();
            let mut fragments = cache_entry.fragments;
            fragments.sort_by_key(|a| {
                let (h, _) = Ipv4Header::parse(a).unwrap();
                h.flags_fragment_offset.get() & 0x1FFF
            });

            let (first_header, _) = Ipv4Header::parse(&fragments[0]).unwrap();
            let header_len = first_header.header_length();
            let total_len = header_len + cache_entry.total_len;
            let mut reassembled = vec![0u8; total_len];

            reassembled[..header_len].copy_from_slice(&fragments[0][..header_len]);

            for frag in &fragments {
                let (header, payload) = Ipv4Header::parse(frag).unwrap();
                let offset = (header.flags_fragment_offset.get() & 0x1FFF) * 8;
                let start = header_len + offset as usize;
                reassembled[start..start + payload.len()].copy_from_slice(payload);
            }

            let reassembled_header =
                Ipv4Header::mut_from_bytes(&mut reassembled[..header_len]).unwrap();
            reassembled_header.total_length.set(total_len as u16);
            reassembled_header.flags_fragment_offset.set(0);
            reassembled_header.header_checksum.set(0);

            return Some(reassembled);
        }

        None
    }
}
