// Copyright 2023-2025 The Android Open Source Project

//! Chip model and management.
//!
//! This module defines the core data structures for representing chips in
//! Netsim, including their types, state, and communication channels. It handles
//! the lifecycle of chips, including creation, updates, and deletion.

use std::fmt;

pub use netsim_types::ChipKind;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::{
    chip_error::ChipError,
    client_error::ClientError,
    device::{DeviceId, Orientation, Position},
    stats::NetsimRadioStats,
};

/// Maps the model ChipKind to the stats RadioKind.
pub fn chip_kind_to_radio_kind(kind: ChipKind) -> crate::stats::RadioKind {
    use crate::stats::RadioKind;
    match kind {
        ChipKind::BLUETOOTH => RadioKind::BluetoothLowEnergy,
        ChipKind::WIFI => RadioKind::Wifi,
        ChipKind::UWB => RadioKind::Uwb,
        ChipKind::NFC => RadioKind::Nfc,
        _ => RadioKind::Unspecified,
    }
}
pub fn chip_kind_to_proto(kind: ChipKind) -> netsim_proto::stats::netsim_radio_stats::Kind {
    use netsim_proto::stats::netsim_radio_stats::Kind;
    match kind {
        ChipKind::BLUETOOTH => Kind::BLUETOOTH_LOW_ENERGY,
        ChipKind::WIFI => Kind::WIFI,
        ChipKind::UWB => Kind::UWB,
        ChipKind::NFC => Kind::NFC,
        _ => Kind::UNSPECIFIED,
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Radio {
    pub state: Option<bool>,
    pub range: f32,
    pub tx_count: u64,
    pub rx_count: u64,
}

pub use crate::packet_streamer::{PacketSink, PacketStream};

// CHIP SERVICE
//
// This module implements the actor model for managing simulated chips.
// There is one chip service actor for each network type (Bluetooth, UWB,
// Wi-Fi).
//
// The "service" is the actor's message-processing loop, which would be
// implemented in a separate task that owns the `mpsc::Receiver<ChipRequest>`.
//
// The "client" (`ChipClient`) is a handle to the actor that allows other parts
// of the system to send messages to it.

/// A unique identifier for a simulated chip, represented as a u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ChipId(pub u32);

impl From<ChipId> for u32 {
    fn from(id: ChipId) -> Self {
        id.0
    }
}

impl From<u32> for ChipId {
    fn from(id: u32) -> Self {
        ChipId(id)
    }
}
/// A reply channel for sending the result of an operation back to the caller.
pub type Responder<T> = oneshot::Sender<Result<T, ChipError>>;

/// Defines the message protocol for the chip service actor.
///
/// Each variant corresponds to a specific command that can be sent to the
/// service. For commands that require a response, a `Responder` channel is
/// included.
#[derive(Debug)]
pub enum ChipRequest {
    /// Create a new chip.
    Create {
        /// The parameters for the new chip.
        params: ChipCreate,
        /// The channel to send the result.
        respond_to: Responder<()>,
    },
    /// Get the state of a chip.
    Read {
        /// The ID of the chip to retrieve.
        id: ChipId,
        /// The channel to send the chip's state back on.
        respond_to: Responder<Chip>,
    },
    /// Update an existing chip.
    Update {
        /// The ID of the chip to patch.
        id: ChipId,
        /// The patch to apply to the chip.
        patch: ChipUpdate,
        /// The channel to send the updated chip state back on.
        respond_to: Responder<Chip>,
    },
    /// Delete a chip.
    Delete {
        /// The ID of the chip to delete.
        id: ChipId,
        /// The channel to send the operation result back on.
        respond_to: Responder<()>,
    },
    /// Reset all event counters for a chip.
    Reset {
        /// The ID of the chip to reset.
        id: ChipId,
    },
    /// Get radio statistics for all chips.
    GetStatistics {
        /// The channel to send the statistics back on.
        respond_to: Responder<Box<[NetsimRadioStats]>>,
    },
    /// Get the total number of chips for testing purposes.
    GetCountForTesting {
        /// The channel to send the count back on.
        respond_to: Responder<usize>,
    },
    /// Command the service to shut down gracefully.
    Shutdown,
}

/// The top-level parameters for creating any kind of chip.
///
/// This struct provides all the necessary information for creating a new
/// simulated chip, including its ID, packet transport, and technology-specific
/// configurations. It is used in the [`ChipRequest::Create`] variant and
/// passed to the chip service through the [`ChipClient::create`] method.
pub struct ChipCreate {
    // TODO: This could be inside Chip
    /// A unique identifier for the new chip.
    pub id: ChipId,
    /// The transport for packet input.
    pub packet_stream: Option<PacketStream>,
    /// The transport for packet output.
    pub packet_sink: Option<PacketSink>,
    // TODO: Use Chip instead
    /// Chip config.
    pub config: ChipConfig,
    /// The ID of the device this chip belongs to.
    pub device_id: DeviceId,
}

impl fmt::Debug for ChipCreate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChipCreate")
            .field("id", &self.id)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

// TODO: use Chip instead
#[derive(Debug, Clone)]
/// Chip configuration
pub struct ChipConfig {
    /// The name of the chip.
    pub name: String,
    /// The manufacturer of the chip.
    pub manufacturer: String,
    /// The product name of the chip.
    pub product_name: String,
    /// Technology-specific parameters.
    pub chip_kind_params: ChipKindParams,
}

impl ChipConfig {
    /// Creates a new `ChipConfig`.
    pub fn new(
        name: impl Into<String>,
        manufacturer: impl Into<String>,
        product_name: impl Into<String>,
        chip_kind_params: ChipKindParams,
    ) -> Self {
        ChipConfig {
            name: name.into(),
            manufacturer: manufacturer.into(),
            product_name: product_name.into(),
            chip_kind_params,
        }
    }
}

impl From<&ChipKindParams> for ChipKind {
    fn from(params: &ChipKindParams) -> Self {
        match params {
            ChipKindParams::Bluetooth(bt) => match bt.mode {
                crate::bluetooth::BluetoothMode::Beacon(_) => ChipKind::BLUETOOTH,
                _ => ChipKind::BLUETOOTH,
            },
            ChipKindParams::Wifi(_) => ChipKind::WIFI,
            ChipKindParams::Uwb(_) => ChipKind::UWB,
            ChipKindParams::Cell(_) => ChipKind::CELLULAR,
            ChipKindParams::Ap(_) => ChipKind::AP,
        }
    }
}

/// An enum holding the parameters for a specific chip technology.
#[derive(Debug, Clone)]
pub enum ChipKindParams {
    /// Bluetooth parameters.
    Bluetooth(crate::bluetooth::BluetoothCreate),
    /// Wi-Fi parameters.
    Wifi(crate::wifi::WifiCreate),
    /// UWB parameters.
    Uwb(crate::uwb::UwbCreate),
    /// Cellular parameters.
    Cell(crate::cell::CellCreate),
    /// Access Point parameters.
    Ap(crate::ap::ApCreate),
}

pub use crate::{
    ap::{Ap, ApCreate, ApUpdate, WifiMode},
    bluetooth::{
        beacon::BleBeacon, BeaconParams, Bluetooth, BluetoothCreate, BluetoothMode,
        BluetoothUpdate, DeviceParams, ScannerParams,
    },
    cell::{Cell, CellCreate},
    uwb::{Uwb, UwbCreate, UwbUpdate},
    wifi::{Wifi, WifiCreate, WifiUpdate},
};

impl fmt::Display for ChipId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ======================================================================
// Chip - All stateful fields for a chip
// ======================================================================

/// The generic representation of a simulated chip.
///
/// A major purpose of the `Chip` wrapper class is:
/// 1. To store common fields for all chip kinds.
/// 2. To allow common signatures (the `ChipActor`) for clients.
///
/// # Architecture
///
/// The model follows a **Component - Variant** pattern:
///
/// 1. **`Chip` (The Entity)**: Contains generic fields common to all chips,
///    such as `id`, `name`, `position`, and `device_id`.
/// 2. **`ChipVariant` (The Dispatcher)**: The `variant` field is an enum that
///    strictly owns the technology-specific struct (e.g.,
///    `Bluetooth(Bluetooth)`).
///
/// Use `Chip` for generic operations (positioning, lifecycle) and access
/// `variant` for technology-specific state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Chip {
    pub id: u32,
    pub kind: ChipKind,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub position: Position,
    pub orientation: Orientation,
    pub device_id: DeviceId,
    pub variant: Option<ChipVariant>,
    pub links: Vec<(ChipId, i8)>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl Chip {
    pub fn is_le_enabled(&self) -> bool {
        if let Some(ChipVariant::Bluetooth(bt)) = &self.variant {
            return bt.low_energy.state.unwrap_or(true);
        }
        true
    }

    pub fn is_classic_enabled(&self) -> bool {
        if let Some(ChipVariant::Bluetooth(bt)) = &self.variant {
            return bt.classic.state.unwrap_or(true);
        }
        true
    }

    pub fn is_uwb_enabled(&self) -> bool {
        matches!(
            self.variant,
            Some(ChipVariant::Uwb(Uwb { radio: Radio { state: Some(true) | None, .. } }))
        )
    }
}

fn default_enabled() -> bool {
    true
}

/// Information about a chip, including technology-specific details.
///
/// # Component Composition
///
/// Each variant owns a dedicated struct from a technology-specific module.
/// These structs often compose or wrap the generic `Radio` struct.
///
/// - **Bluetooth**: Has two `Radio` components (`classic` and `low_energy`).
/// - **Wifi**: Wraps a single `Radio` component.
/// - **Uwb**: Wraps a single `Radio` component.
/// - **Cell**: Contains cellular-specific state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChipVariant {
    Bluetooth(crate::bluetooth::Bluetooth),
    Wifi(crate::wifi::Wifi),
    Uwb(crate::uwb::Uwb),
    Cell(crate::cell::Cell),
    Ap(crate::ap::Ap),
}

impl From<ChipKind> for ChipVariant {
    fn from(kind: ChipKind) -> Self {
        match kind {
            ChipKind::BLUETOOTH => ChipVariant::Bluetooth(crate::bluetooth::Bluetooth {
                low_energy: Default::default(),
                classic: Default::default(),
            }),
            ChipKind::WIFI => ChipVariant::Wifi(Default::default()),
            ChipKind::UWB => ChipVariant::Uwb(Default::default()),
            ChipKind::CELLULAR => ChipVariant::Cell(crate::cell::Cell { state: "unknown".into() }),
            ChipKind::AP => ChipVariant::Ap(crate::ap::Ap {
                config: Default::default(),
                associations: Vec::new(),
            }),
            // Use Bluetooth as fallback for generic/unknown types if necessary,
            // or panic if this is unreachable. For now, default to Bluetooth for unimplemented
            // types.
            _ => ChipVariant::Bluetooth(Default::default()),
        }
    }
}

// ======================================================================
// ChipUpdate - All patchable fields for a chip
// ======================================================================

/// This struct represents the partial, optional set of changes to a
/// Chip provided by the client.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChipUpdate {
    pub id: Option<ChipId>,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub position: Option<Position>,
    pub orientation: Option<Orientation>,
    pub variant: Option<ChipVariantUpdate>,
    pub links: Option<Vec<(ChipId, i8)>>,
    pub enabled: Option<bool>,
}

/// Generic radio chip update (Bluetooth, Wi-Fi, UWB).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RadioUpdate {
    pub state: Option<bool>,
}

impl RadioUpdate {
    pub fn apply(&self, radio: &mut Radio) {
        if let Some(state) = self.state {
            radio.state = Some(state);
        }
    }
}

/// The techbology variant specific fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChipVariantUpdate {
    Bluetooth(crate::bluetooth::BluetoothUpdate),
    Wifi(crate::wifi::WifiUpdate),
    Uwb(crate::uwb::UwbUpdate),
    Ap(crate::ap::ApUpdate),
}

impl ChipVariantUpdate {
    pub fn kind(&self) -> ChipKind {
        match self {
            ChipVariantUpdate::Bluetooth(_) => ChipKind::BLUETOOTH,
            ChipVariantUpdate::Wifi(_) => ChipKind::WIFI,
            ChipVariantUpdate::Uwb(_) => ChipKind::UWB,
            ChipVariantUpdate::Ap(_) => ChipKind::AP,
        }
    }
}

// =============================================================================
// Chip Actor CLIENT
// =============================================================================

/// A client handle for interacting with the chip server actor.
///
/// There is one chip server actor for each network type (Bluetooth, UWB,
/// Wi-Fi). This client provides a high-level API for sending `ChipRequest`
/// messages to the server over an `mpsc` channel. It abstracts away the channel
/// and `oneshot` responder boilerplate for each command.

/// A generic client for interacting with any chip server actor (UWB, WiFi,
/// Cell).
#[derive(Clone)]
pub struct RadioChipClient {
    sender: tokio::sync::mpsc::Sender<ChipRequest>,
}

impl RadioChipClient {
    pub fn new(sender: tokio::sync::mpsc::Sender<ChipRequest>) -> Self {
        Self { sender }
    }
}

impl std::fmt::Debug for RadioChipClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioChipClient").finish_non_exhaustive()
    }
}

#[cfg_attr(feature = "testing", mockall::automock)]
#[async_trait::async_trait]
impl ChipClient for RadioChipClient {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ChipRequest::Create { params, respond_to: tx })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?.map_err(ClientError::Chip)
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ChipRequest::Read { id, respond_to: tx })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?.map_err(ClientError::Chip)
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ChipRequest::Update { id, patch, respond_to: tx })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?.map_err(ClientError::Chip)
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ChipRequest::Delete { id, respond_to: tx })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?.map_err(ClientError::Chip)
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ChipRequest::GetStatistics { respond_to: tx })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?.map_err(ClientError::Chip)
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ChipRequest::GetCountForTesting { respond_to: tx })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        rx.await.map_err(|e| ClientError::Recv(e.to_string()))?.map_err(ClientError::Chip)
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.sender
            .send(ChipRequest::Shutdown)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        Ok(())
    }

    async fn reset(&self, id: ChipId) -> Result<(), ClientError> {
        self.sender
            .send(ChipRequest::Reset { id })
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}

/// A client handle for interacting with the chip server actor.
///
/// There is one chip server actor for each network type (Bluetooth, UWB,
/// Wi-Fi). This client provides a high-level API for sending `ChipRequest`
/// messages to the server over an `mpsc` channel. It abstracts away the channel
/// and `oneshot` responder boilerplate for each command.
#[cfg_attr(feature = "testing", mockall::automock)]
#[async_trait::async_trait]
pub trait ChipClient: std::fmt::Debug + Send + Sync {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError>;
    async fn read(&self, id: ChipId) -> Result<Chip, ClientError>;
    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError>;
    async fn delete(&self, id: ChipId) -> Result<(), ClientError>;
    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError>;
    async fn read_count_for_testing(&self) -> Result<usize, ClientError>;
    async fn shutdown(&self) -> Result<(), ClientError>;
    /// Resets the state of the specified chip.
    async fn reset(&self, id: ChipId) -> Result<(), ClientError>;
    fn clone_box(&self) -> Box<dyn ChipClient>;
    async fn get_global_stats(&self) -> Result<Option<Vec<u8>>, ClientError> {
        Ok(None)
    }
}

impl Clone for Box<dyn ChipClient> {
    fn clone(&self) -> Box<dyn ChipClient> {
        self.clone_box()
    }
}
