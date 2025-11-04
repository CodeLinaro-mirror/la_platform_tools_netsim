// Copyright 2025 The Android Open Source Project

//! Tests for device lifecycle management.
//!
//! This module contains unit tests for the device server's lifecycle operations,
//! including device creation, chip creation success/failure, server shutdown on idle,
//! and listing devices.
//!
//! List of tests:
//! - `test_ps_create_success`: Verifies successful device and chip creation.
//! - `test_ps_create_chip_failure`: Ensures proper error handling when chip creation fails.
//! - `test_server_shutdown_on_idle`: Checks if the server correctly shuts down after an idle period.
//! - `test_list_devices`: Confirms that devices are correctly listed after creation.

use devices::server::Server;
use netsim_api::{
    chip_error::ChipError,
    chips::{ChipClient, ChipConfig, ChipRequest, NetworkParams},
    client_error::ClientError,
    device_error::DeviceError,
    devices::{CreateDeviceParams, DeviceClient, DeviceConfig},
};
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};

/// Encapsulates the common setup for a test environment.
struct TestFixture {
    client: DeviceClient,
    _server_task: JoinHandle<()>,
    chip_rx: mpsc::Receiver<ChipRequest>,
}

/// Sets up a test environment with a running server and a client.
fn setup() -> TestFixture {
    let (chip_tx, chip_rx) = mpsc::channel(10);
    let bt_client = ChipClient::new(chip_tx);
    let (server, client) = Server::new(bt_client);
    let server_task = tokio::spawn(server.run());
    TestFixture { client, _server_task: server_task, chip_rx }
}

/// Sets up a test environment for idle shutdown tests with custom timeouts.
fn setup_for_idle_test() -> (DeviceClient, JoinHandle<()>) {
    let (chip_tx, _) = mpsc::channel(10);
    let bt_client = ChipClient::new(chip_tx);
    let (mut server, client) = Server::new(bt_client);
    server.start_timeout = Duration::from_millis(10);
    server.idle_timeout = Duration::from_millis(10);
    let server_task = tokio::spawn(server.run());
    (client, server_task)
}

/// Helper to create a default `CreateDeviceParams` for tests.
fn create_test_ps_device_params() -> CreateDeviceParams {
    CreateDeviceParams {
        device_guid: "test_guid".to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig {
            name: "test_device".to_string(),
            visible: true,
            position: Default::default(),
            orientation: Default::default(),
        },
        chip_config: create_chip_config(netsim_api::chips::BluetoothMode::Device(
            netsim_api::chips::DeviceParams {},
        )),
    }
}

fn create_chip_config(mode: netsim_api::chips::BluetoothMode) -> ChipConfig {
    ChipConfig::new(
        "ps_chip",
        "ps_manufacturer",
        "ps_product",
        NetworkParams::Bluetooth(netsim_api::chips::BluetoothParams {
            address: "11:22:33:44:55:66".to_string(),
            bt_properties: netsim_proto::configuration::Controller::default(),
            mode,
        }),
    )
}

/// Helper to mock the chip service's response for a single `Create` request.
async fn mock_chip_service_response(
    mut chip_rx: mpsc::Receiver<ChipRequest>,
    response: Result<(), ChipError>,
) {
    if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
        respond_to.send(response).unwrap();
    }
}

#[tokio::test]
async fn test_ps_create_success() {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with success.
    tokio::spawn(mock_chip_service_response(chip_rx, Ok(())));

    let result = client.ps_create(create_test_ps_device_params()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ps_create_chip_failure() {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with an error.
    tokio::spawn(mock_chip_service_response(
        chip_rx,
        Err(ChipError::InvalidArguments("Mock chip creation failure".to_string())),
    ));

    let result = client.ps_create(create_test_ps_device_params()).await;

    match result {
        Err(ClientError::Device(e)) => match *e {
            DeviceError::Chip(ChipError::InvalidArguments(msg)) => {
                assert_eq!(msg, "Mock chip creation failure");
            }
            _ => panic!("Unexpected device error type: {:?}", e),
        },
        _ => panic!("Unexpected client error type: {:?}", result),
    }
}

#[tokio::test]
async fn test_server_shutdown_on_idle() {
    let (client, server_task) = setup_for_idle_test();

    // Wait for longer than the timeout for the server to shut down.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(server_task.is_finished());

    // Any subsequent command should fail because the server is down.
    let result = client.ps_create(create_test_ps_device_params()).await;
    assert!(matches!(result, Err(ClientError::Send(_))));
}

#[tokio::test]
async fn test_list_devices() {
    test_list_devices_inner().await;
}
async fn test_list_devices_inner() {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with success.
    tokio::spawn(mock_chip_service_response(chip_rx, Ok(())));

    let result = client.ps_create(create_test_ps_device_params()).await;
    assert!(result.is_ok());

    let list_result = client.list().await;
    assert!(list_result.is_ok());

    let response = list_result.unwrap();
    assert_eq!(response.devices.len(), 1);

    let device = &response.devices[0];
    let device_config = DeviceConfig {
        name: "test_device".to_string(),
        visible: true,
        position: Default::default(),
        orientation: Default::default(),
    };
    assert_eq!(device.name, "test_guid");
    assert_eq!(device.visible, Some(device_config.visible));
    assert_eq!(device.position.as_ref(), Some(&device_config.position));
    assert_eq!(device.orientation.as_ref(), Some(&device_config.orientation));

    // TODO: Verify device.chips once the field is populated.
}
