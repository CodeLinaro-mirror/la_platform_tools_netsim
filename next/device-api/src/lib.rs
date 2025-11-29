//! # Netsim Device API
//!
//! This crate defines the public API for the Netsim device service.
//! It includes the behavioral contract (actions) for the `device-actor`.
//!
//! ## Rationale
//!
//! Prior to this refactoring, the device service's API was tightly coupled with its implementation. This led to several issues:
//! - **Circular Dependencies**: Crates needing to interact with the device service often had to depend on the entire `device-actor` or `netsim-model` in a way that created complex dependency graphs.
//! - **Lack of Encapsulation**: Internal implementation details of the device service were sometimes exposed to consumers.
//! - **Testing Difficulty**: Mocking the device service for integration tests was more difficult than necessary due to the coupled nature of the API.
//!
//! By introducing a dedicated `device-api` crate, we create a clear boundary between the service's contract and its implementation.
//!
//! ## Architecture
//!
//! ### New Structure
//!
//! The refactoring introduces the following structure:
//!
//! - **`netsim-model`**: Remains the base layer for data models (e.g., `Device`, `Chip`, `Position`). It does not depend on `device-api`.
//! - **`device-api` (New)**: Defines the behavioral contract for the device service.
//!     - Depends on: `netsim-model`
//!     - Contains: `DeviceAction` (enum of supported operations), `DeviceActionResult`, and re-exports of relevant models from `netsim-model` for convenience.
//! - **`device-actor`**: Implements the device service.
//!     - Depends on: `device-api`, `netsim-model`, and other service crates.
//!     - Implements the handling logic for `DeviceAction`.
//! - **Consumer Crates** (e.g., `wifi`, `cell`, `bluetooth`, `netsim-client`):
//!     - Depend on: `device-api` (and `netsim-model` if needed).
//!     - Do not depend directly on `device-actor` for API definitions.
//!
//! ### Dependency Graph (Simplified)
//!
//! ```text
//! graph TD
//!     ConsumerCrates[Consumer Crates<br>(wifi, cell, bluetooth, client)] --> DeviceAPI[device-api]
//!     ConsumerCrates --> NetsimModel[netsim-model]
//!     DeviceActor[device-actor] --> DeviceAPI
//!     DeviceActor --> NetsimModel
//!     DeviceAPI --> NetsimModel
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
