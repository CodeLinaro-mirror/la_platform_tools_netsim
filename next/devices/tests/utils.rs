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
        DeviceParams, NetworkKind, NetworkParams,
    },
    devices::{api, DeviceClient, DeviceConfig, DevicePsCreate},
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};

/// Encapsulates the common setup for a test environment.
pub struct TestFixture {
    pub client: DeviceClient,
    pub chip_rx: mpsc::Receiver<ChipRequest>,
    pub server_task: JoinHandle<()>,
}

// TODO: Simplify tests by using a mock that records the messages
// received by the server and has a method to fetch those messages
fn mock_chip_client() -> (ChipClient, mpsc::Receiver<ChipRequest>) {
    let (chip_tx, chip_rx) = mpsc::channel(10);
    (ChipClient::new(chip_tx), chip_rx)
}

/// Sets up a test environment with a running server and a client.
pub fn setup() -> TestFixture {
    let (server, device_client) = Server::new(false);
    let (bt_client, chip_rx) = mock_chip_client();
    let mut chip_clients = HashMap::new();
    chip_clients.insert(NetworkKind::Bluetooth, bt_client.clone());
    let server_task = tokio::spawn(server.run(chip_clients));
    TestFixture { client: device_client, chip_rx, server_task }
}

/// Sets up a test environment for idle shutdown tests with custom timeouts.
pub fn setup_for_idle_test() -> TestFixture {
    let (server, device_client) =
        Server::new_with_timeouts(Duration::from_millis(10), Duration::from_millis(10));
    let (bt_client, chip_rx) = mock_chip_client();
    let mut chip_clients = HashMap::new();
    chip_clients.insert(NetworkKind::Bluetooth, bt_client.clone());
    let server_task = tokio::spawn(server.run(chip_clients));
    TestFixture { client: device_client, chip_rx, server_task }
}

/// Helper to create a default `DevicePsCreate` for tests.
pub fn create_test_ps_device_params() -> DevicePsCreate {
    DevicePsCreate {
        device_guid: "test_guid".to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig::new(
            "test_device",
            true,
            Default::default(),
            Default::default(),
        ),
        chip_config: create_chip_config(BluetoothMode::Device(DeviceParams::default())),
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

/// Helper to mock the chip service's response for a single request.
pub async fn mock_chip_service_response(
    mut chip_rx: mpsc::Receiver<ChipRequest>,
    response: Result<(), ChipError>,
) {
    if let Some(msg) = chip_rx.recv().await {
        match msg {
            ChipRequest::Create { respond_to, .. } => respond_to.send(response).unwrap(),
            ChipRequest::Delete { respond_to, .. } => respond_to.send(response).unwrap(),
            _ => panic!("Unexpected ChipRequest variant in mock"),
        }
    }
}

pub fn get_test_create_device_request(device_name: String) -> api::DeviceCreate {
    let chip_create = api::ChipConfig {
        name: "beacon".to_string(),
        manufacturer: "Netsim".to_string(),
        product_name: "NetsimBeacon".to_string(),
        chip: api::Chip::Beacon(BleBeacon::default()),
    };

    api::DeviceCreate {
        device_config: DeviceConfig::new(device_name, true, Default::default(), Default::default()),
        chip: chip_create,
    }
}
