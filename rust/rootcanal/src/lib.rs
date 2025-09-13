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

#![warn(missing_docs)]

//! # Bluetooth Controller
//!
//! This crate provides a simplified interface for interacting with a Bluetooth
//! controller. It is designed to be used in testing and simulation scenarios.
//!
//! The main entry point is the [`Bluetooth`] struct, which represents the
//! Bluetooth subsystem. It provides methods for creating and managing
//! Bluetooth controllers.
//!
// The [`bluetooth`] module contains definitions for common Bluetooth data
//! types, such as addresses, device classes, and features.
//!
//! The [`error`] module defines the error types used in this crate.
//!
//! The [`ffi`] module contains the low-level bindings to the C++ FFI. This
//! module is not intended to be used directly by consumers of this crate.
//!
//! ## Example
//!

pub mod bluetooth;
pub mod controller;
pub mod error;
pub mod ffi;
pub mod types;

pub use bluetooth::Bluetooth;
pub use controller::{Id, Stats};
pub use types::{Address, Idc, Phy};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::Callbacks as ControllerCallbacks;
    use crate::types::{Address, Idc};
    use std::ffi::c_int;
    use std::str::FromStr;

    struct MockBluetoothCallbacks;
    impl bluetooth::Callbacks for MockBluetoothCallbacks {
        fn on_send_ll(
            &self,
            _source_id: u32,
            _destination_id: u32,
            _packet: &[u8],
            _phy: Phy,
            tx_power: i32,
        ) -> Option<i32> {
            Some(tx_power)
        }
    }

    struct MockControllerCallbacks;
    impl ControllerCallbacks for MockControllerCallbacks {
        fn send_hci(&self, _source_id: Id, _idc: Idc, _data: &[u8]) {}
        fn send_ll(&self, _source_id: Id, _packet: &[u8], _phy: Phy, _tx_power: i32) {}
        fn invalid_packet_received(
            &self,
            _source_id: Id,
            _reason: c_int,
            _message: &str,
            _data: &[u8],
        ) {
        }
    }

    #[test]
    fn test_create_and_delete_controller() {
        let bluetooth = Bluetooth::new(Box::new(MockBluetoothCallbacks));
        let address = Address::from_str("01:02:03:04:05:06").unwrap();
        let id = bluetooth.new_controller(address, Box::new(MockControllerCallbacks));

        assert_eq!(bluetooth.len(), 1);

        bluetooth.remove_controller(id).unwrap();
        assert_eq!(bluetooth.len(), 0);
    }
}
