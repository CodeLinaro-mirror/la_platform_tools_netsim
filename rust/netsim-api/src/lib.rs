// Copyright 2023-2025 The Android Open Source Project

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

/// Chip error definitions.
pub mod chip_error;
/// Chip actor definitions.
pub mod chips;
/// Device actor definitions.
pub mod devices;
/// Macros for the client methods.
pub(crate) mod macros;
/// Packet streamer definitions.
pub mod packet_streamer;
