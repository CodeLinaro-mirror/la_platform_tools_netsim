//! # Netsim Device API
//!
//! This crate defines the public API for the Netsim device service.
//! It includes the behavioral contract (actions) for the `device-actor`.
//!
//! ## Architecture
//!
//! ### Structure
//!
//! The device service is structured as follows:
//!
//! - **`netsim-model`**: Remains the base layer for data models (e.g., `Device`, `Chip`, `Position`). It does not depend on `device-api`.
//! - **`device-api`**: Defines the behavioral contract for the device service.
//!     - Depends on: `netsim-model`
//!     - Contains: `DeviceAction` (enum of supported operations), `DeviceActionResult`, and re-exports of relevant models from `netsim-model` for convenience.
//! - **`device-actor`**: Implements the device service.
//!     - Depends on: `device-api`, `netsim-model`, and other service crates.
//!     - Implements the handling logic for `DeviceAction`.
//! - **`netsim-client`**: Provides a client-side wrapper for the device service.
//!     - Depends on: `device-api`, `netsim-model`.
//! - **Consumer Crates** (e.g., `wifi`, `cell`, `bluetooth`):
//!     - Depend on: `device-api`, `netsim-client` (and `netsim-model` if needed).
//!     - Do not depend directly on `device-actor` for API definitions.
//!
//! ### Dependency Graph (Simplified)
//!
//! ```text
//! graph TD
//!     ConsumerCrates[Consumer Crates<br>(wifi, cell, bluetooth)] --> NetsimClient[netsim-client]
//!     ConsumerCrates --> DeviceAPI[device-api]
//!     NetsimClient --> DeviceAPI
//!     DeviceActor[device-actor] --> DeviceAPI
//!     DeviceActor --> NetsimModel[netsim-model]
//!     DeviceAPI --> NetsimModel
//!     NetsimClient --> NetsimModel
//! ```
//!
//! ## API Definition
//!
//! The `device-api` crate defines the following key components:
//!
//! ### `DeviceAction`
//!
//! An enum representing the actions that can be performed on the device service. This serves as the primary interface for consumers.
//!
//! ### Re-exports
//!
//! To simplify consumption, `device-api` re-exports commonly used types from `netsim-model`.
//!
//! ## Benefits
//!
//! 1.  **Clearer Boundaries**: The separation between API and implementation is now explicit.
//! 2.  **Reduced Coupling**: Consumer crates only depend on the API, not the implementation.
//! 3.  **Improved Build Times**: Changes to the `device-actor` implementation do not require recompiling consumer crates, as long as the API remains stable.
//! 4.  **Easier Testing**: Mocking the device service is now a matter of implementing the `DeviceAction` contract.

use netsim_model::chip::{ChipId, PacketSink, PacketStream};
use netsim_model::device::api::DeviceChipCreate;
use serde::{Deserialize, Serialize};
use std::fmt;

// Re-export key data models from netsim-model for convenience.
pub use netsim_model::device::api::{DeviceCreate, DeviceUpdate, ListDeviceResponse};
pub use netsim_model::device::DeviceAddChip;
pub use netsim_model::device::{api, Device, DeviceConfig, DeviceId, Orientation, Position};

// Behavioral Contract

pub enum DeviceAction {
    Reset,
    NotifyChipRemoved(DeviceId, ChipId),
    AddChip {
        chip_config: DeviceChipCreate,
        packet_stream: Option<PacketStream>,
        packet_sink: Option<PacketSink>,
    },
}

impl fmt::Debug for DeviceAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceAction::Reset => write!(f, "Reset"),
            DeviceAction::NotifyChipRemoved(device_id, chip_id) => {
                f.debug_tuple("NotifyChipRemoved").field(device_id).field(chip_id).finish()
            }
            DeviceAction::AddChip { chip_config, .. } => f
                .debug_struct("AddChip")
                .field("chip_config", chip_config)
                .field("packet_stream", &"Option<PacketStream>")
                .field("packet_sink", &"Option<PacketSink>")
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceActionResult {
    Success,
    ChipId(ChipId),
}
