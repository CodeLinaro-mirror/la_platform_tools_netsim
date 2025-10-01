// Copyright 2023-2025 The Android Open Source Project
#![allow(missing_docs)]

//! A shared, dependency-free crate that defines the shared data structures
//! and command enums for the netsim control plane.

use async_trait::async_trait;
use std::fmt;
use thiserror::Error;

/// The error type for the netsim API.
#[derive(Error, Debug)]
pub enum Error {
    /// An error occurred during I/O.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// An error occurred during packet processing.
    #[error("Packet processing error: {0}")]
    Packet(String),
    /// The operation is not supported.
    #[error("Unsupported operation")]
    Unsupported,
}

/// A transport-agnostic abstraction for bidirectional packet I/O.
///
/// This trait is used by the simulation to communicate with the client (e.g.,
/// an Android Virtual Device) without being tied to a specific transport like
/// gRPC or file descriptors.
#[async_trait]
pub trait PacketStreamerApi: Send + Sync {
    /// Asynchronously reads the next packet from the client.
    /// Returns `Ok(None)` if the stream is closed gracefully.
    async fn read_packet(&mut self) -> Result<Option<Vec<u8>>, Error>;

    /// Asynchronously writes a packet to the client.
    async fn write_packet(&mut self, packet: Vec<u8>) -> Result<(), Error>;
}

/// The top-level command enum for all netsim simulation control commands.
#[derive(Debug)]
pub enum SimCommand {
    /// Command to create a new chip.
    CreateChip(CreateChipParams),
    /// Command to patch an existing chip.
    PatchChip(PatchChipParams),
    /// Command to get information about a chip.
    GetChip(GetChipParams),
    /// Command to delete a chip.
    DeleteChip(DeleteChipParams),
}

/// A placeholder for the data used to patch a chip.
#[derive(Debug, Clone, Default)]
pub struct ChipPatch {
    // TODO: Add fields for patching, e.g., position, radio state, etc.
}

/// The parameters for the `PatchChip` command.
#[derive(Debug, Clone)]
pub struct PatchChipParams {
    /// The ID of the chip to patch.
    pub chip_id: u32,
    /// The patch to apply to the chip.
    pub patch: ChipPatch,
}

/// The parameters for the `GetChip` command.
#[derive(Debug, Clone)]
pub struct GetChipParams {
    /// The ID of the chip to get.
    pub chip_id: u32,
}

/// The parameters for the `DeleteChip` command.
#[derive(Debug, Clone)]
pub struct DeleteChipParams {
    /// The ID of the chip to delete.
    pub chip_id: u32,
}

/// The parameters for the `CreateChip` command.
pub struct CreateChipParams {
    /// The parameters for the chip to create.
    pub chip_params: ChipParams,
    /// The packet streamer to use for the chip.
    pub packet_streamer: Box<dyn PacketStreamerApi>,
}

impl fmt::Debug for CreateChipParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateChipParams")
            .field("chip_params", &self.chip_params)
            .field("packet_streamer", &"...")
            .finish()
    }
}

/// The parameters that are common to all chip kinds.
#[derive(Debug, Clone)]
pub enum ChipParams {
    /// A full-featured Bluetooth controller that connects to a remote host
    /// (e.g., an Android Virtual Device).
    ///
    /// This chip type streams HCI command and event packets over the
    /// `PacketStreamerApi`.
    BluetoothDevice(BluetoothDeviceParams),
    /// A simple, simulated Bluetooth beacon that only sends advertisements.
    BluetoothBeacon(BeaconCreationParams),
    /// A passive Bluetooth sniffer that receives all link-layer traffic.
    ///
    /// This chip type streams raw link-layer packets over the
    /// `PacketStreamerApi`.
    BluetoothSniffer(BluetoothSnifferParams),
}

/// The parameters for creating a Bluetooth virtual device.
#[derive(Debug, Clone, Default)]
pub struct BluetoothDeviceParams {
    pub address: String,
    // TODO: Add rootcanal controller properties when the dependency is available.
}

/// The parameters for creating a Beacon chip.
#[derive(Debug, Clone, Default)]
pub struct BeaconCreationParams {
    /// The Bluetooth address of the beacon.
    pub address: String,
    /// The advertising settings for the beacon.
    pub settings: AdvertiseSettings,
    /// The advertising data for the beacon.
    pub adv_data: AdvertiseData,
    /// The scan response data for the beacon.
    pub scan_response: AdvertiseData,
}

/// The parameters for creating a BluetoothSniffer chip.
#[derive(Debug, Clone, Default)]
pub struct BluetoothSnifferParams {
    // TODO: Add sniffer-specific parameters.
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseSettings.AdvertiseMode`
#[derive(Debug, Clone, Default)]
pub enum AdvertiseMode {
    /// Advertise in low power mode.
    #[default]
    LowPower,
    /// Advertise in balanced mode.
    Balanced,
    LowLatency,
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseSettings.AdvertiseTxPower`
#[derive(Debug, Clone, Default)]
pub enum AdvertiseTxPower {
    UltraLow,
    #[default]
    Low,
    Medium,
    High,
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseSettings`
#[derive(Debug, Clone, Default)]
pub struct AdvertiseSettings {
    pub interval_ms: u64,
    pub tx_power_dbm: i32,
    pub scannable: bool,
    pub timeout: u64,
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseData.Service`
#[derive(Debug, Clone, Default)]
pub struct Service {
    pub uuid: String,
    pub data: Vec<u8>,
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseData`
#[derive(Debug, Clone, Default)]
pub struct AdvertiseData {
    pub include_device_name: bool,
    pub include_tx_power_level: bool,
    pub manufacturer_data: Vec<u8>,
    pub services: Vec<Service>,
}
