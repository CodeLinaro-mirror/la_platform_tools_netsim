// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{devices::chip::ChipIdentifier, ffi::ffi_bluetooth::RustBluetoothChip};
use cxx::{UniquePtr, let_cxx_string};

/// Rust bluetooth chip trait.
pub trait RustBluetoothChipCallbacks {
    fn tick(&mut self);

    // TODO: include the pdl library in Rust for reading the packet contents.
    fn receive_link_layer_packet(
        &mut self,
        source_address: String,
        destination_address: String,
        packet_type: u8,
        packet: &[u8],
    );
}

/// AddRustDeviceResult for the returned object of AddRustDevice() in C++
pub struct AddRustDeviceResult {
    pub rust_chip: UniquePtr<RustBluetoothChip>,
    pub facade_id: u32,
}

pub fn create_add_rust_device_result(
    facade_id: u32,
    rust_chip: UniquePtr<RustBluetoothChip>,
) -> Box<AddRustDeviceResult> {
    Box::new(AddRustDeviceResult { facade_id, rust_chip })
}

/// Add a bluetooth chip by an object implements RustBluetoothChipCallbacks trait.
pub fn rust_bluetooth_add(
    chip_id: ChipIdentifier,
    callbacks: Box<dyn RustBluetoothChipCallbacks>,
    string_type: String,
    address: String,
) -> Box<AddRustDeviceResult> {
    let_cxx_string!(cxx_string_type = string_type);
    let_cxx_string!(cxx_address = address);
    crate::ffi::ffi_bluetooth::bluetooth_add_rust_device(
        chip_id.0,
        Box::new(callbacks),
        &cxx_string_type,
        &cxx_address,
    )
}
