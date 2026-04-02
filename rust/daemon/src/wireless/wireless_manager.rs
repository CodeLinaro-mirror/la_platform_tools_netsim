// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    devices::chip::ChipIdentifier,
    wireless::WirelessChipImpl,
    wireless::{ble_beacon, mocked},
};

#[cfg(not(test))]
use crate::wireless::{bluetooth, uwb, wifi_chip, wifi_manager};

/// Parameter for each constructor of Emulated Chips
#[allow(clippy::large_enum_variant, dead_code)]
pub enum CreateParam {
    BleBeacon(ble_beacon::CreateParams),
    #[cfg(not(test))]
    Bluetooth(bluetooth::CreateParams),
    #[cfg(not(test))]
    Wifi(wifi_chip::CreateParams),
    #[cfg(not(test))]
    Uwb(uwb::CreateParams),
    Mock(mocked::CreateParams),
}

/// This is called when the transport module receives a new packet stream
/// connection from a virtual device.
pub fn add_chip(create_param: &CreateParam, chip_id: ChipIdentifier) -> WirelessChipImpl {
    // Based on create_param, construct WirelessChip.
    match create_param {
        CreateParam::BleBeacon(params) => ble_beacon::add_chip(params, chip_id),
        #[cfg(not(test))]
        CreateParam::Bluetooth(params) => bluetooth::add_chip(params, chip_id),
        #[cfg(not(test))]
        CreateParam::Wifi(params) => wifi_manager::add_chip(params, chip_id),
        #[cfg(not(test))]
        CreateParam::Uwb(params) => uwb::add_chip(params, chip_id),
        CreateParam::Mock(params) => mocked::add_chip(params, chip_id),
    }
}
