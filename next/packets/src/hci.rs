// Copyright 2026 The Android Open Source Project

//! Defines structures for representing Bluetooth HCI packets using `zerocopy`.
//!
//! This module provides definitions for HCI commands, events, and subevents,
//! focusing on the subset required for Bluetooth beacon simulation and basic
//! scanning tests.

use zerocopy::{
    byteorder::LittleEndian, FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, U16, U64,
};

/// HCI OpCodes for Command packets.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct OpCode(pub U16<LittleEndian>);

impl OpCode {
    pub const RESET: Self = Self(U16::from_bytes([0x03, 0x0C]));
    pub const SET_EVENT_MASK: Self = Self(U16::from_bytes([0x01, 0x0C]));
    pub const LE_SET_EVENT_MASK: Self = Self(U16::from_bytes([0x01, 0x20]));
    pub const LE_SET_ADVERTISING_PARAMETERS: Self = Self(U16::from_bytes([0x06, 0x20]));
    pub const LE_SET_ADVERTISING_DATA: Self = Self(U16::from_bytes([0x08, 0x20]));
    pub const LE_SET_SCAN_RESPONSE_DATA: Self = Self(U16::from_bytes([0x09, 0x20]));
    pub const LE_SET_ADVERTISING_ENABLE: Self = Self(U16::from_bytes([0x0A, 0x20]));
    pub const LE_SET_SCAN_PARAMETERS: Self = Self(U16::from_bytes([0x0B, 0x20]));
    pub const LE_SET_SCAN_ENABLE: Self = Self(U16::from_bytes([0x0C, 0x20]));

    pub fn get(self) -> u16 {
        self.0.get()
    }
}

/// HCI Event Codes for Event packets.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct EventCode(pub u8);

impl EventCode {
    pub const COMMAND_COMPLETE: Self = Self(0x0E);
    pub const COMMAND_STATUS: Self = Self(0x0F);
    pub const LE_META_EVENT: Self = Self(0x3E);
}

/// HCI LE Subevent Codes for LE Meta Events.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct SubeventCode(pub u8);

impl SubeventCode {
    pub const LE_ADVERTISING_REPORT: Self = Self(0x02);
}

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

/// HCI Command Packet Header.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct HciCommandHeader {
    pub op_code: OpCode,
    pub parameter_total_length: u8,
}

/// HCI Event Packet Header.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct HciEventHeader {
    pub event_code: EventCode,
    pub parameter_total_length: u8,
}

/// HCI LE Meta Event Header (follows HciEventHeader if event_code is
/// LE_META_EVENT).
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct HciLeMetaEventHeader {
    pub subevent_code: SubeventCode,
}

/// Trait for HCI commands to associate them with their OpCode.
pub trait HciCommand {
    const OP_CODE: OpCode;
}

/// Set Event Mask Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct SetEventMask {
    pub event_mask: U64<LittleEndian>,
}

impl HciCommand for SetEventMask {
    const OP_CODE: OpCode = OpCode::SET_EVENT_MASK;
}

/// LE Set Event Mask Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetEventMask {
    pub le_event_mask: U64<LittleEndian>,
}

impl HciCommand for LeSetEventMask {
    const OP_CODE: OpCode = OpCode::LE_SET_EVENT_MASK;
}

/// LE Set Advertising Parameters Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetAdvertisingParameters {
    pub advertising_interval_min: U16<LittleEndian>,
    pub advertising_interval_max: U16<LittleEndian>,
    pub advertising_type: AdvertisingType,
    pub own_address_type: OwnAddressType,
    pub peer_address_type: PeerAddressType,
    pub peer_address: Address,
    pub advertising_channel_map: u8,
    pub advertising_filter_policy: AdvertisingFilterPolicy,
}

impl HciCommand for LeSetAdvertisingParameters {
    const OP_CODE: OpCode = OpCode::LE_SET_ADVERTISING_PARAMETERS;
}

/// LE Set Advertising Data Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetAdvertisingData {
    pub advertising_data_length: u8,
    pub advertising_data: [u8; 31],
}

impl HciCommand for LeSetAdvertisingData {
    const OP_CODE: OpCode = OpCode::LE_SET_ADVERTISING_DATA;
}

/// LE Set Scan Response Data Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetScanResponseData {
    pub advertising_data_length: u8,
    pub advertising_data: [u8; 31],
}

impl HciCommand for LeSetScanResponseData {
    const OP_CODE: OpCode = OpCode::LE_SET_SCAN_RESPONSE_DATA;
}

/// LE Set Advertising Enable Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetAdvertisingEnable {
    pub advertising_enable: Enable,
}

impl HciCommand for LeSetAdvertisingEnable {
    const OP_CODE: OpCode = OpCode::LE_SET_ADVERTISING_ENABLE;
}

/// LE Set Scan Parameters Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetScanParameters {
    pub le_scan_type: LeScanType,
    pub le_scan_interval: U16<LittleEndian>,
    pub le_scan_window: U16<LittleEndian>,
    pub own_address_type: OwnAddressType,
    pub scanning_filter_policy: LeScanningFilterPolicy,
}

impl HciCommand for LeSetScanParameters {
    const OP_CODE: OpCode = OpCode::LE_SET_SCAN_PARAMETERS;
}

/// LE Set Scan Enable Command.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct LeSetScanEnable {
    pub le_scan_enable: Enable,
    pub filter_duplicates: Enable,
}

impl HciCommand for LeSetScanEnable {
    const OP_CODE: OpCode = OpCode::LE_SET_SCAN_ENABLE;
}

/// Reset Command (Empty payload).
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug)]
pub struct Reset;

impl HciCommand for Reset {
    const OP_CODE: OpCode = OpCode::RESET;
}
