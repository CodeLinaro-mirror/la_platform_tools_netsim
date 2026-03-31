// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Bindings for rootcanal/models/controller/ffi.h

#![allow(non_camel_case_types)]

/// A C-compatible unsigned 8-bit integer.
pub type uint8_t = u8;
/// A C-compatible size type.
pub type size_t = usize;

use std::{
    option::Option,
    os::raw::{c_char, c_int, c_void},
};

extern "C" {
    /// Creates a new Bluetooth controller.
    pub fn ffi_controller_new(
        address: *const uint8_t,
        send_hci: Option<
            unsafe extern "C" fn(
                cookie: *mut c_void,
                idc: c_int,
                data: *const uint8_t,
                data_len: size_t,
            ),
        >,
        send_ll: Option<
            unsafe extern "C" fn(
                cookie: *mut c_void,
                data: *const uint8_t,
                data_len: size_t,
                phy: c_int,
                tx_power: c_int,
            ),
        >,
        invalid_packet_handler: Option<
            unsafe extern "C" fn(
                cookie: *mut c_void,
                reason: c_int,
                message: *const c_char,
                data: *const uint8_t,
                data_len: size_t,
            ),
        >,
        ranging_estimator: Option<
            unsafe extern "C" fn(cookie1: *mut c_void, cookie2: *mut c_void) -> u32,
        >,
        cookie: *mut c_void,
        proto_bytes: *const uint8_t,
        proto_len: size_t,
    ) -> *mut c_void;

    /// Deletes a Bluetooth controller.
    pub fn ffi_controller_delete(controller: *mut c_void);

    /// Receives an HCI packet from the host.
    pub fn ffi_controller_receive_hci(
        controller: *mut c_void,
        idc: c_int,
        data: *const uint8_t,
        data_len: size_t,
    );

    /// Receives a link layer packet from a peer.
    pub fn ffi_controller_receive_ll(
        controller: *mut c_void,
        data: *const uint8_t,
        data_len: size_t,
        phy: c_int,
        rssi: c_int,
    );

    /// Advances the controller's state by one tick.
    pub fn ffi_controller_tick(controller: *mut c_void);

}
