// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! A DNS packet builder.

use zerocopy::FromBytes;

use super::dns::DnsHeader;

/// A builder for a DNS packet.
pub struct DnsPacketBuilder<'a> {
    header: &'a mut DnsHeader,
    payload: &'a mut [u8],
    offset: usize,
}

impl<'a> DnsPacketBuilder<'a> {
    /// Creates a new `DnsPacketBuilder`.
    pub fn new(buffer: &'a mut [u8]) -> Option<Self> {
        if buffer.len() < std::mem::size_of::<DnsHeader>() {
            return None;
        }
        let (header_slice, payload) = buffer.split_at_mut(std::mem::size_of::<DnsHeader>());
        header_slice.fill(0); // Zero-initialize the header!
        let header = DnsHeader::mut_from_bytes(header_slice).ok()?;
        Some(Self { header, payload, offset: 0 })
    }

    /// Sets the transaction ID.
    pub fn transaction_id(&mut self, id: u16) {
        self.header.transaction_id = id.into();
    }

    /// Sets the flags.
    pub fn flags(&mut self, flags: u16) {
        self.header.flags = flags.into();
    }

    /// Adds a question to the packet. Returns `None` if the buffer is too small
    /// or name is invalid.
    pub fn add_question(&mut self, name: &str, qtype: u16, qclass: u16) -> Option<()> {
        let original_offset = self.offset;
        if self.write_name(name).is_none() {
            self.offset = original_offset;
            return None;
        }
        if self.offset + 4 > self.payload.len() {
            self.offset = original_offset;
            return None;
        }
        self.payload[self.offset..self.offset + 2].copy_from_slice(&qtype.to_be_bytes());
        self.offset += 2;
        self.payload[self.offset..self.offset + 2].copy_from_slice(&qclass.to_be_bytes());
        self.offset += 2;
        self.header.num_questions = (self.header.num_questions.get() + 1).into();
        Some(())
    }

    fn write_name(&mut self, name: &str) -> Option<usize> {
        let name = name.strip_suffix('.').unwrap_or(name);
        if name.is_empty() {
            // Root domain: just write the null terminator (0)
            if self.offset + 1 > self.payload.len() {
                return None;
            }
            self.payload[self.offset] = 0;
            self.offset += 1;
            return Some(1);
        }
        let mut len = 0;
        for label in name.split('.') {
            if label.is_empty() {
                return None; // Intermediate empty labels (e.g. "foo..bar") are invalid!
            }
            let label_len = label.len();
            if label_len > 63 {
                return None; // DNS label limit
            }
            if self.offset + label_len + 1 > self.payload.len() {
                return None;
            }
            self.payload[self.offset] = label_len as u8;
            self.offset += 1;
            self.payload[self.offset..self.offset + label_len].copy_from_slice(label.as_bytes());
            self.offset += label_len;
            len += label_len + 1;
        }
        if self.offset + 1 > self.payload.len() {
            return None;
        }
        self.payload[self.offset] = 0;
        self.offset += 1;
        Some(len + 1)
    }

    /// Builds the DNS packet.
    pub fn build(self) -> usize {
        std::mem::size_of::<DnsHeader>() + self.offset
    }
}
