// Copyright 2026 The Android Open Source Project

use std::fmt;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Generic enable/disable.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct Enable(pub u8);

impl Enable {
    pub const DISABLED: Self = Self(0x00);
    pub const ENABLED: Self = Self(0x01);
}

/// Advertising Type for LE Set Advertising Parameters.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct AdvertisingType(pub u8);

impl AdvertisingType {
    pub const ADV_IND: Self = Self(0x00);
    pub const ADV_DIRECT_IND_HIGH: Self = Self(0x01);
    pub const ADV_SCAN_IND: Self = Self(0x02);
    pub const ADV_NONCONN_IND: Self = Self(0x03);
    pub const ADV_DIRECT_IND_LOW: Self = Self(0x04);
}

/// Own Address Type.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct OwnAddressType(pub u8);

impl OwnAddressType {
    pub const PUBLIC_DEVICE_ADDRESS: Self = Self(0x00);
    pub const RANDOM_DEVICE_ADDRESS: Self = Self(0x01);
    pub const RESOLVABLE_OR_PUBLIC_ADDRESS: Self = Self(0x02);
    pub const RESOLVABLE_OR_RANDOM_ADDRESS: Self = Self(0x03);
}

/// Peer Address Type.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct PeerAddressType(pub u8);

impl PeerAddressType {
    pub const PUBLIC_DEVICE_OR_IDENTITY_ADDRESS: Self = Self(0x00);
    pub const RANDOM_DEVICE_OR_IDENTITY_ADDRESS: Self = Self(0x01);
}

/// Advertising Filter Policy.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct AdvertisingFilterPolicy(pub u8);

impl AdvertisingFilterPolicy {
    pub const ALL_DEVICES: Self = Self(0x00);
    pub const LISTED_SCAN: Self = Self(0x01);
    pub const LISTED_CONNECT: Self = Self(0x02);
    pub const LISTED_SCAN_AND_CONNECT: Self = Self(0x03);
}

/// LE Scan Type.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct LeScanType(pub u8);

impl LeScanType {
    pub const PASSIVE: Self = Self(0x00);
    pub const ACTIVE: Self = Self(0x01);
}

/// LE Scanning Filter Policy.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct LeScanningFilterPolicy(pub u8);

impl LeScanningFilterPolicy {
    pub const ACCEPT_ALL: Self = Self(0x00);
    pub const FILTER_ACCEPT_LIST_ONLY: Self = Self(0x01);
    pub const CHECK_INITIATORS_IDENTITY: Self = Self(0x02);
    pub const FILTER_ACCEPT_LIST_AND_INITIATORS_IDENTITY: Self = Self(0x03);
}

/// Standard 6-byte Bluetooth Address.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(C)]
pub struct Address {
    pub bytes: [u8; 6],
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.bytes[5],
            self.bytes[4],
            self.bytes[3],
            self.bytes[2],
            self.bytes[1],
            self.bytes[0]
        )
    }
}
