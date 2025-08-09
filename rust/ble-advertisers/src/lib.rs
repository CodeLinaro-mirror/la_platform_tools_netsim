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

//! A library for creating and managing Bluetooth Low Energy (BLE) advertisers.
//!
//! This crate provides a high-level API for building legacy advertising and scan
//! response packets and for managing the scheduling of multiple advertisers in a
//! thread-safe manner.
//!
//! # Key Features
//!
//! * **Packet Construction**: Build custom BLE advertising and scan response packets
//!   with support for common data types like device name, TX power, manufacturer
//!   data, and GATT services.
//! * **Thread-Safe Scheduling**: The `Advertisers` allows you to manage multiple
//!   advertisers concurrently from different async tasks or threads.
//! * **Event-Driven API**: Provides a simple `duration()` method to determine the
//!   wait time until the next advertising event, making it easy to integrate into
//!   an event loop.
//!
//! # Example
//!
//! ```
//! use ble_advertisers::advertiser::Advertiser;
//! use ble_advertisers::advertisers::Advertisers;
//! use ble_advertisers::advertise_settings::{AdvertiseMode, AdvertiseSettingsBuilder, TxPowerLevel};
//! use ble_advertisers::advertise_data::{AdvertiseData, Service};
//! use std::time::Duration;
//!
//! // 1. Configure the advertiser's settings and data
//! let settings = AdvertiseSettingsBuilder::new()
//!     .mode(AdvertiseMode::new(Duration::from_millis(150)))
//!     .build();
//!
//! let data = AdvertiseData::builder("MyDevice".to_string(), TxPowerLevel::default())
//!     .include_device_name()
//!     .build()
//!     .unwrap();
//!
//! // 2. Create an advertiser
//! let address = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
//! let advertiser = Advertiser::new(settings, data, None, address).unwrap();
//!
//! // 3. Manage advertisers in a thread-safe set
//! let set = Advertisers::new();
//! set.add(1, advertiser);
//!
//! // 4. In your event loop, get the next event time and packet
//! if let Some((id, next_event_in)) = set.duration() {
//!     // Sleep until the next event
//!     // std::thread::sleep(next_event_in);
//!
//!     // Get the packet to send
//!     if let Some(packet) = set.packet(id) {
//!         // Send the packet over the air
//!         // println!("Sending packet for advertiser {}: {:?}", id, packet);
//!     }
//! }
//! ```

pub mod advertise_data;
pub mod advertise_settings;
pub mod advertiser;
pub mod advertisers;
pub mod error;

/// Low-level BLE link layer packet definitions.
pub mod link_layer;
