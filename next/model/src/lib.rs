// Copyright 2023-2025 The Android Open Source Project

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

//! # Netsim Model
//!
//! This crate defines the core data structures and types used across the Netsim project.
//! It serves as the base layer for the dependency graph.
//!
//! ## Dependency Rules
//!
//! - **Base Layer**: This crate should not depend on other Netsim crates (e.g., `*-api`, `*-actor`).
//! - **Shared Types**: It contains shared types like `ChipId`, `Position`, and `Device` that are used by both API and implementation crates.
//!
//! ## Dependency Hierarchy
//!
//! The Netsim project uses a layered architecture to prevent circular dependencies:
//!
//! 1.  **Level 0: Base Layer** (`netsim-model`)
//!     - Shared data types (e.g., `ChipId`, `Position`).
//!     - No dependencies on other Netsim crates.
//! 2.  **Level 1: API Layer** (`*-api`)
//!     - Behavioral contracts (Actions, Results, Traits).
//!     - Depends on: `netsim-model`.
//! 3.  **Level 2: Implementation & Client Layer**
//!     - `*-actor`: Implements the contracts. Depends on `*-api`, `netsim-model`.
//!     - `netsim-client`: Client-side wrappers. Depends on `*-api`, `netsim-model`.
//! 4.  **Level 3: Consumer Layer** (`daemon`, `wifi`, `cell`, etc.)
//!     - Uses the services. Depends on `netsim-client`, `*-api`.
//!
//! By maintaining this structure, we ensure a stable foundation and prevent circular dependencies.

/// Bluetooth related definitions.
pub mod bluetooth;

/// Chip configuration parameters.
pub mod ap;
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
pub mod link;
/// Macros for the client methods.
pub mod macros;
/// Packet streamer definitions.
pub mod packet_streamer;
/// Statistics related definitions.
pub mod stats;

pub use initial_info::{Chip, ChipInfo, ChipKind, DeviceInfo};
