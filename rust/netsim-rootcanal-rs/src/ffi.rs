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

//! Bindings for rootcanal/models/controller/ffi.h

#![allow(non_camel_case_types)]

/// A C-compatible unsigned 8-bit integer.
pub type uint8_t = u8;
/// A C-compatible size type.
pub type size_t = usize;

use std::option::Option;
use std::os::raw::{c_char, c_int, c_void};

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
        cookie: *mut c_void,
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
