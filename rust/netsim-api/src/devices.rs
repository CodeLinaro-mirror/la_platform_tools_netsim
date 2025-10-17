// Copyright 2023-2025 The Android Open Source Project

use netsim_proto::frontend::{ListDeviceResponse, PatchDeviceRequest};
use netsim_proto::model::Device;
use tokio::sync::oneshot;

// DEVICE SERVICE

/// Defines the message protocol for the device service actor.
pub enum DeviceCommand {
    /// Create a new device.
    CreateDevice {
        /// The parameters for the new device.
        device: Device,
        /// The channel to send the created device back on.
        responder: oneshot::Sender<Device>,
    },
    /// Patch an existing device.
    PatchDevice {
        /// The patch to apply to the device.
        request: PatchDeviceRequest,
    },
    /// List all devices.
    ListDevice {
        /// The channel to send the list of devices back on.
        responder: oneshot::Sender<ListDeviceResponse>,
    },
}

/// A message to get the version of the device service.
#[allow(dead_code)]
pub struct GetVersionMessage {
    response: oneshot::Sender<String>,
}

/// The parameters for creating a Bluetooth virtual device.
#[derive(Debug, Clone, Default)]
pub struct BluetoothDeviceParams {
    /// The Bluetooth address of the device.
    pub address: String,
    // TODO: Add rootcanal controller properties when the dependency is available.
}

/// Corresponds to `netsim.model.Chip.BleBeacon.AdvertiseData.Service`
#[derive(Debug, Clone, Default)]
pub struct Service {
    /// The UUID of the service.
    pub uuid: String,
    /// The data associated with the service.
    pub data: Vec<u8>,
}
