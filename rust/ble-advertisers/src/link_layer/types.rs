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

/// A 6-byte bluetooth device address.
pub type Address = [u8; 6];

/// The type of a bluetooth device address.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum AddressKind {
    Public = 0x00,
    Random = 0x01,
}

/// The PDU type for a legacy advertising packet.
///
/// As defined in the Bluetooth Core Specification, Vol 4, Part E, Section 7.8.5.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum LegacyAdvertisingType {
    AdvInd = 0x00,
    AdvDirectInd = 0x01,
    AdvNonconnInd = 0x02,
    ScanReq = 0x03,
    ScanRsp = 0x04,
    ConnectInd = 0x05,
    AdvScanInd = 0x06,
}
