// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! This module defines common Bluetooth data types, such as addresses.

use std::{ffi::c_int, str::FromStr};

use crate::error::{Error, Result};

/// A Bluetooth address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address {
    /// The 6-byte address.
    pub address: [u8; 6],
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(Error::MalformedAddress(s.to_string()));
        }
        let mut address = [0; 6];
        for (i, part) in parts.iter().enumerate() {
            address[i] = u8::from_str_radix(part, 16)?;
        }
        Ok(Self { address })
    }
}

impl Address {
    /// Returns the address as a byte slice.
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.address
    }

    /// Returns `true` if the address is resolvable.
    pub fn is_resolvable(&self) -> bool {
        (self.address[0] & 0xc0) == 0x40
    }

    /// Returns `true` if the address is non-resolvable.
    pub fn is_non_resolvable(&self) -> bool {
        (self.address[0] & 0xc0) == 0x00
    }

    /// Returns `true` if the address is a static identity address.
    pub fn is_static_identity(&self) -> bool {
        (self.address[0] & 0xc0) == 0xc0
    }
}

/// The physical layer.
#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Phy {
    /// The Low Energy physical layer.
    LowEnergy = 0,
    /// The BR/EDR physical layer.
    BrEdr = 1,
}

impl From<c_int> for Phy {
    fn from(item: c_int) -> Self {
        match item {
            0 => Phy::LowEnergy,
            1 => Phy::BrEdr,
            _ => panic!("Unknown PHY value"),
        }
    }
}
