// Copyright 2023-2025 The Android Open Source Project
#![allow(missing_docs)]

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

pub mod chip_error;
pub mod chips;
pub mod devices;
pub(crate) mod macros;
pub mod packet_streamer;
