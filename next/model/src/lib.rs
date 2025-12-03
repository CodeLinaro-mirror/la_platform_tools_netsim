// Copyright 2023-2025 The Android Open Source Project

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

/// Bluetooth related definitions.
pub mod bluetooth;
pub mod capture;

/// Chip actor definitions.
pub mod chip;
/// Chip error definitions.
pub mod chip_error;
pub mod client_error;
/// Device actor definitions.
pub mod device;
pub mod device_error;
/// Structs for initial connection handshake.
pub mod initial_info;
/// Macros for the client methods.
pub(crate) mod macros;
/// Packet streamer definitions.
pub mod packet_streamer;
/// Statistics related definitions.
pub mod stats;

pub use initial_info::{Chip, ChipInfo, ChipKind, DeviceInfo};
