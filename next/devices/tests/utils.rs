// Copyright 2025 The Android Open Source Project

//! Utility functions for device lifecycle tests.
//!
//! This module provides helper functions for setting up test environments,
//! creating test data, and mocking responses for device server tests.

use devices::server::Server;
use netsim_api::{
    chip_error::ChipError,
    chips::{
        BleBeacon, BluetoothMode, BluetoothParams, ChipClient, ChipConfig, ChipRequest,
        DeviceParams, NetworkParams,
    },
    devices::{api, CreateDeviceParams, DeviceClient, DeviceConfig},
};
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};

/// Encapsulates the common setup for a test environment.
pub struct TestFixture {
    pub client: DeviceClient,
    pub server_task: JoinHandle<()>,
    pub chip_rx: mpsc::Receiver<ChipRequest>,
}

/// Sets up a test environment with a running server and a client.
pub fn setup() -> TestFixture {
    let (chip_tx, chip_rx) = mpsc::channel(10);
    // TODO: Replace with MockChipServer to reduce boilerplate. It could remember the last command received.
    let bt_client = ChipClient::new(chip_tx);
    let (server, client) = Server::new(bt_client);
    let server_task = tokio::spawn(server.run());
    TestFixture { client, server_task, chip_rx }
}

/// Sets up a test environment for idle shutdown tests with custom timeouts.
pub fn setup_for_idle_test() -> TestFixture {
    let (chip_tx, chip_rx) = mpsc::channel(10);
    let bt_client = ChipClient::new(chip_tx);
    let (server, client) =
        Server::new_with_timeouts(bt_client, Duration::from_millis(10), Duration::from_millis(10));
    let server_task = tokio::spawn(server.run());
    TestFixture { client, server_task, chip_rx }
}

/// Helper to create a default `CreateDeviceParams` for tests.
pub fn create_test_ps_device_params() -> CreateDeviceParams {
    CreateDeviceParams {
        device_guid: "test_guid".to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig::new(
            "test_device",
            true,
            Default::default(),
            Default::default(),
        ),
        chip_config: create_chip_config(BluetoothMode::Device(DeviceParams {})),
    }
}

pub fn create_chip_config(mode: BluetoothMode) -> ChipConfig {
    ChipConfig::new(
        "ps_chip",
        "ps_manufacturer",
        "ps_product",
        NetworkParams::Bluetooth(BluetoothParams {
            address: "11:22:33:44:55:66".to_string(),
            bt_properties: netsim_api::bluetooth::Controller::default(),
            mode,
        }),
    )
}

/// Helper to mock the chip service's response for a single `Create` request.
pub async fn mock_chip_service_response(
    mut chip_rx: mpsc::Receiver<ChipRequest>,
    response: Result<(), ChipError>,
) {
    if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
        respond_to.send(response).unwrap();
    }
}

pub fn get_test_create_device_request(device_name: String) -> api::DeviceCreate {
    let chip_create = api::ChipCreate {
        name: "beacon".to_string(),
        manufacturer: "Netsim".to_string(),
        product_name: "NetsimBeacon".to_string(),
        chip: api::Chip::Beacon(BleBeacon::default()),
    };

    api::DeviceCreate {
        config: DeviceConfig::new(device_name, true, Default::default(), Default::default()),
        chip: chip_create,
    }
}
