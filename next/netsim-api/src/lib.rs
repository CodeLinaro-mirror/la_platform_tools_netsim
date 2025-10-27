// Copyright 2023-2025 The Android Open Source Project

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

/// Chip error definitions.
pub mod chip_error;
/// Chip actor definitions.
pub mod chips;
pub mod client_error;
pub mod device_error;
/// Device actor definitions.
pub mod devices;
/// Structs for initial connection handshake.
pub mod initial_info;
/// Macros for the client methods.
pub(crate) mod macros;
/// Packet streamer definitions.
pub mod packet_streamer;

pub use initial_info::{Chip, ChipInfo, ChipKind, DeviceInfo};
