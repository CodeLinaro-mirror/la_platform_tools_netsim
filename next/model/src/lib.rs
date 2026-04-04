// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

//! # Netsim Model
//!
//! This crate defines the core data structures and types used across the Netsim
//! project. It serves as the base layer for the dependency graph.
//!
//! ## Dependency Rules
//!
//! - **Base Layer**: This crate should not depend on other Netsim crates (e.g.,
//!   `*-api`, `*-actor`).
//! - **Shared Types**: It contains shared types like `ChipId`, `Position`, and
//!   `Device` that are used by both API and implementation crates.
//!
//! ## Dependency Hierarchy
//!
//! The Netsim project uses a layered architecture to prevent circular
//! dependencies:
//!
//! 1. **Level 0: Base Layer** (`netsim-model`)
//!     - Shared data types (e.g., `ChipId`, `Position`).
//!     - No dependencies on other Netsim crates.
//! 2. **Level 1: API Layer** (`*-api`)
//!     - Behavioral contracts (Actions, Results, Traits).
//!     - Depends on: `netsim-model`.
//! 3. **Level 2: Implementation & Client Layer**
//!     - `*-actor`: Implements the contracts. Depends on `*-api`,
//!       `netsim-model`.
//!     - `netsim-client`: Client-side wrappers. Depends on `*-api`,
//!       `netsim-model`.
//! 4. **Level 3: Consumer Layer** (`daemon`, `wifi`, `cell`, etc.)
//!     - Uses the services. Depends on `netsim-client`, `*-api`.
//!
//! By maintaining this structure, we ensure a stable foundation and prevent
//! circular dependencies.

pub(crate) mod ap;
pub(crate) mod bluetooth;
pub(crate) mod cell;
pub(crate) mod chip;
pub(crate) mod chip_error;
pub(crate) mod client_error;
pub(crate) mod device;
pub(crate) mod device_error;
pub(crate) mod initial_info;
pub(crate) mod link;
pub(crate) mod macros;
pub(crate) mod packet_streamer;
pub(crate) mod stats;
pub(crate) mod uwb;
pub(crate) mod wifi;

// Explicit Facade Exports

// From chip.rs
// From ap.rs
// From bluetooth.rs
// From cell.rs
#[cfg(any(test, feature = "testing"))]
pub use crate::chip::MockChipClient;
// From client_error.rs
pub use crate::client_error::ClientError;
// From device::api
pub use crate::device::api::{
    DeviceChip, DeviceChipCreate, DeviceCreate, DeviceUpdate, ListDeviceResponse, PoseUpdate,
};
// From device.rs
pub use crate::device::{
    Device, DeviceAddChip, DeviceConfig, DeviceId, DeviceInfo, Orientation, Pose, Position,
};
// From device_error.rs
pub use crate::device_error::DeviceError;
// From initial_info.rs
pub use crate::initial_info::{ChipInfo, ChipKind};
// From link.rs
pub use crate::link::{Link, LinkId, LinkUpdate};
// From stats.rs
pub use crate::stats::{NetsimDeviceStats, NetsimFrontendStats, NetsimRadioStats, RadioKind};
pub use crate::{
    ap::{Ap, ApCreate, ApUpdate, WifiMode, DEFAULT_WIFI_BSSID, DEFAULT_WIFI_SSID},
    bluetooth::{
        beacon::{
            AdvertiseData, AdvertiseMode, AdvertiseSettings, AdvertiseTxPower, BleBeacon, Interval,
            Service, TxPower,
        },
        BeaconParams, Bluetooth, BluetoothCreate, BluetoothMode, BluetoothUpdate, Controller,
        DeviceParams, ScannerParams, SnifferParams,
    },
    cell::{Cell, CellCreate, ModemAction, RegistrationStatus},
    chip::{
        chip_kind_to_proto, chip_kind_to_radio_kind, Chip, ChipClient, ChipConfig, ChipCreate,
        ChipId, ChipKindParams, ChipRequest, ChipUpdate, ChipVariant, ChipVariantUpdate,
        PacketSink, PacketStream, Radio, RadioChipClient, RadioUpdate,
    },
    chip_error::ChipError,
    uwb::{Uwb, UwbCreate, UwbUpdate},
    wifi::{Wifi, WifiCreate, WifiUpdate},
};
