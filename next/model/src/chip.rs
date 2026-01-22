// Copyright 2023-2025 The Android Open Source Project

//! Chip model and management.
//!
//! This module defines the core data structures for representing chips in Netsim,
//! including their types, state, and communication channels. It handles the lifecycle
//! of chips, including creation, updates, and deletion.

use crate::bluetooth::beacon::{AdvertiseData, AdvertiseSettings};
use crate::bluetooth::Controller as RootcanalController;
use crate::chip_error::ChipError;
use crate::client_error::ClientError;

use crate::device::{DeviceId, Orientation, Position};
use crate::stats::NetsimRadioStats;
use bytes::Bytes;
use futures::Sink;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::pin::Pin;
use tokio::sync::oneshot;
use tokio_stream::Stream;

/// The kind of network technology the chip supports.
///
/// This enumeration is used to distinguish between different types of simulated
/// radios and to route packets to the correct handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ChipKind {
    #[default]
    UNSPECIFIED = 0,
    BLUETOOTH = 1,
    WIFI = 2,
    UWB = 3,
    NFC = 4,
    BleBeacon = 5,
    CELLULAR = 6,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Radio {
    pub state: Option<bool>,
    pub range: f32,
    pub tx_count: i32,
    pub rx_count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChipType {
    Bt(crate::bluetooth::Bluetooth),
    BleBeacon(crate::bluetooth::beacon::BleBeacon),
    Uwb(Radio),
    Wifi(Radio),
}

// The only error from PacketStream occurs when source closes connection.
/// A stream of packets from the chip.
pub type PacketStream = Box<dyn Stream<Item = Bytes> + Send + Sync + Unpin>;
/// A sink for packets to the chip.
pub type PacketSink = Pin<Box<dyn Sink<Bytes, Error = std::io::Error> + Send + Sync>>;

// CHIP SERVICE
//
// This module implements the actor model for managing simulated chips.
// There is one chip service actor for each network type (Bluetooth, UWB, Wi-Fi).
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
    pub network_params: NetworkParams,
}

impl ChipConfig {
    /// Creates a new `ChipConfig`.
    pub fn new(
        name: impl Into<String>,
        manufacturer: impl Into<String>,
        product_name: impl Into<String>,
        network_params: NetworkParams,
    ) -> Self {
        ChipConfig {
            name: name.into(),
            manufacturer: manufacturer.into(),
            product_name: product_name.into(),
            network_params,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkKind {
    Bluetooth,
    Wifi,
    Uwb,
    Cell,
}

impl From<&NetworkParams> for NetworkKind {
    fn from(params: &NetworkParams) -> Self {
        match params {
            NetworkParams::Bluetooth(_) => NetworkKind::Bluetooth,
            NetworkParams::Wifi(_) => NetworkKind::Wifi,
            NetworkParams::Uwb(_) => NetworkKind::Uwb,
            NetworkParams::Cell(_) => NetworkKind::Cell,
        }
    }
}

impl From<NetworkKind> for ChipKind {
    fn from(kind: NetworkKind) -> Self {
        match kind {
            NetworkKind::Bluetooth => ChipKind::BLUETOOTH,
            NetworkKind::Wifi => ChipKind::WIFI,
            NetworkKind::Uwb => ChipKind::UWB,
            NetworkKind::Cell => ChipKind::CELLULAR,
        }
    }
}

/// An enum holding the parameters for a specific chip technology.
#[derive(Debug, Clone)]
pub enum NetworkParams {
    /// Bluetooth parameters.
    Bluetooth(BluetoothCreate),
    /// Wi-Fi parameters.
    Wifi(WifiCreate),
    /// UWB parameters.
    Uwb(UwbCreate),
    /// Cellular parameters.
    Cell(CellCreate),
}

/// Parameters for creating a Bluetooth chip.
///
/// This struct holds all the necessary parameters for creating a Bluetooth chip,
/// including its address, controller properties, and operational mode. It is
/// nested within [`ChipCreate`] when the chip being created is a
/// Bluetooth chip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothCreate {
    /// The Bluetooth address of the device.
    pub address: String,
    /// Rootcanal controller properties.
    pub bt_properties: RootcanalController,
    /// The operational mode of the Bluetooth chip.
    pub mode: BluetoothMode,
}

/// An enum to differentiate between the kinds of Bluetooth chips.
///
/// This enum differentiates between the various operational modes of a
/// Bluetooth chip, such as Device, Beacon, and Sniffer. It is used within
/// [`BluetoothCreate`] to specify the chip's behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BluetoothMode {
    /// A full, virtual Bluetooth controller that can be paired with.
    Device(DeviceParams),
    /// A simple, non-interactive BLE beacon that broadcasts advertisements.
    Beacon(Box<BeaconParams>),
    /// A passive Bluetooth sniffer to capture nearby traffic.
    Sniffer(SnifferParams),
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BleBeacon {
    // BD_ADDR address
    pub address: String,
    // Settings on how beacon functions
    pub settings: Option<AdvertiseSettings>,
    // Advertising Data
    pub adv_data: Option<AdvertiseData>,
    // Scan Response Data
    pub scan_response: Option<AdvertiseData>,
}

/// Parameters for creating a virtual Bluetooth device.
///
/// This struct holds parameters for creating a virtual Bluetooth device and is
/// used when the [`BluetoothMode`] is [`BluetoothMode::Device`].
// TODO: Rename to BluetoothDeviceParams to avoid confusion with DeviceConfig
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceParams {}

/// Parameters for creating a BLE beacon.
///
/// This struct holds parameters for creating a BLE beacon and is used when the
/// [`BluetoothMode`] is [`BluetoothMode::Beacon`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BeaconParams {
    /// The BLE beacon's configuration.
    pub ble_beacon: BleBeacon,
}

/// Parameters for a Bluetooth sniffer.
///
/// This struct holds parameters for a Bluetooth sniffer and is used when the
/// [`BluetoothMode`] is [`BluetoothMode::Sniffer`].
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnifferParams {
    // Future sniffer-specific properties can be added here.
}

/// Parameters for creating a Wi-Fi chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiCreate {
    // Future Wi-Fi specific properties.
}

/// Parameters for creating a UWB chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct UwbCreate {
    // Future UWB specific properties.
}

/// Parameters for creating a Cellular chip.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellCreate {
    // Future Cellular specific properties.
}

/// Parameters for the ChipDied message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChipDiedParams {
    /// The ID of the chip that died.
    pub id: ChipId,
}

impl fmt::Display for ChipId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ======================================================================
// Chip - All stateful fields for a chip
// ======================================================================

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
}

/// Information about a chip, including technology-specific details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChipVariant {
    Bluetooth(Bluetooth),
    Wifi(Radio),
    Uwb(Radio),
    Cell(CellChip),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bluetooth {
    pub low_energy: Radio,
    pub classic: Radio,
}

/// Cellular technology specific chip information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellChip {
    /// A string representing the current state of the cellular modem.
    pub state: String,
}

impl From<NetworkKind> for ChipVariant {
    fn from(kind: NetworkKind) -> Self {
        match kind {
            NetworkKind::Bluetooth => ChipVariant::Bluetooth(Bluetooth {
                low_energy: Default::default(),
                classic: Default::default(),
            }),
            NetworkKind::Wifi => ChipVariant::Wifi(Default::default()),
            NetworkKind::Uwb => ChipVariant::Uwb(Default::default()),
            NetworkKind::Cell => ChipVariant::Cell(CellChip { state: "unknown".into() }),
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
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub position: Option<Position>,
    pub orientation: Option<Orientation>,
    pub variant: Option<ChipVariantUpdate>,
    pub links: Option<Vec<(ChipId, i8)>>,
}

/// The techbology variant specific fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChipVariantUpdate {
    Bluetooth(Radio),
    Wifi(Radio),
    Uwb(Radio),
    Cell(CellUpdate),
}

/// Cellular technology specific chip information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellUpdate {
    /// A string representing the current state of the modem.
    pub state: Option<String>,
}

// =============================================================================
// Chip Actor CLIENT
// =============================================================================

/// A client handle for interacting with the chip server actor.
///
/// There is one chip server actor for each network type (Bluetooth, UWB, Wi-Fi).
/// This client provides a high-level API for sending `ChipRequest` messages to
/// the server over an `mpsc` channel. It abstracts away the channel and
/// `oneshot` responder boilerplate for each command.
/// A client handle for interacting with the chip server actor.
///
/// There is one chip server actor for each network type (Bluetooth, UWB, Wi-Fi).
/// This client provides a high-level API for sending `ChipRequest` messages to
/// the server over an `mpsc` channel. It abstracts away the channel and
/// `oneshot` responder boilerplate for each command.
/// A generic client for interacting with any chip server actor (UWB, WiFi, Cell).
#[derive(Clone)]
pub struct RadioChipClient {
    sender: tokio::sync::mpsc::Sender<ChipRequest>,
}

impl RadioChipClient {
    pub fn new(sender: tokio::sync::mpsc::Sender<ChipRequest>) -> Self {
        Self { sender }
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
/// There is one chip server actor for each network type (Bluetooth, UWB, Wi-Fi).
/// This client provides a high-level API for sending `ChipRequest` messages to
/// the server over an `mpsc` channel. It abstracts away the channel and
/// `oneshot` responder boilerplate for each command.
#[cfg_attr(feature = "testing", mockall::automock)]
#[async_trait::async_trait]
pub trait ChipClient: Send + Sync {
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
}

impl Clone for Box<dyn ChipClient> {
    fn clone(&self) -> Box<dyn ChipClient> {
        self.clone_box()
    }
}
