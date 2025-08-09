// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This module defines common Bluetooth data types, such as addresses.

use crate::error::{Error, Result};
use std::ffi::c_int;
use std::str::FromStr;

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

/// The indicator of the packet type.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum Idc {
    /// Host to controller command.
    Cmd = 1,
    /// Bidirectional ACL data.
    Acl = 2,
    /// Bidirectional synchronous data.
    Sco = 3,
    /// Controller to host event.
    Evt = 4,
    /// Bidirectional isochronous data.
    Iso = 5,
}

impl From<c_int> for Idc {
    fn from(item: c_int) -> Self {
        match item {
            1 => Idc::Cmd,
            2 => Idc::Acl,
            3 => Idc::Sco,
            4 => Idc::Evt,
            5 => Idc::Iso,
            _ => panic!("Unknown IDC value"),
        }
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
