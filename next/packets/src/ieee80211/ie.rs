// Copyright 2025 The Android Open Source Project

//! Information Element (IE) parsing for IEEE 802.11 frames.

use std::fmt;

/// Common Information Element Tags (IDs).
pub mod tags {
    pub const SSID: u8 = 0;
    pub const SUPPORTED_RATES: u8 = 1;
    pub const DS_PARAMETER_SET: u8 = 3;
    pub const TRAFFIC_INDICATION_MAP: u8 = 5;
    pub const COUNTRY: u8 = 7;
    pub const IBSS_PARAMETER_SET: u8 = 6;
    pub const ERP_INFORMATION: u8 = 42;
    pub const HT_CAPABILITIES: u8 = 45;
    pub const HT_OPERATION: u8 = 61;
    pub const RSN: u8 = 48; // Robust Security Network
    pub const EXTENDED_SUPPORTED_RATES: u8 = 50;
    pub const VENDOR_SPECIFIC: u8 = 221;
}

/// Represents a parsed Information Element.
#[derive(Clone, Copy)]
pub struct InformationElement<'a> {
    pub id: u8,
    pub length: u8,
    pub body: &'a [u8],
}

impl<'a> fmt::Debug for InformationElement<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InformationElement")
            .field("id", &self.id)
            .field("length", &self.length)
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// Iterator for Information Elements in a byte slice.
pub struct IeIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> IeIterator<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for IeIterator<'a> {
    type Item = InformationElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 2 > self.data.len() {
            return None;
        }

        let id = self.data[self.offset];
        let len = self.data[self.offset + 1] as usize;

        let start = self.offset + 2;
        let end = start + len;

        if end > self.data.len() {
            // Malformed IE or end of buffer truncated?
            // We'll stop iteration to be safe.
            return None;
        }

        let ie = InformationElement { id, length: len as u8, body: &self.data[start..end] };

        self.offset = end;
        Some(ie)
    }
}
