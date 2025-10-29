// Copyright 2025 The Android Open Source Project

//! Tests for device lifecycle management.
//!
//! This module contains unit tests for the device server's lifecycle operations,
//! including device creation, chip creation success/failure, server shutdown on idle,
//! and listing devices.
//!
//! List of tests:
//! - `test_create_device_succeeds`: Verifies successful device and chip creation using the CreateDevice API.
//! - `test_ps_create_success`: Verifies successful device and chip creation using the PsCreate API.
//! - `test_ps_create_chip_failure`: Ensures proper error handling when chip creation fails during a PsCreate call.
//! - `test_server_shutdown_on_idle`: Checks if the server correctly shuts down after a configured idle period.
//! - `test_list_devices`: Confirms that devices are correctly listed after creation using the PsCreate API.

use devices::server::Server;
use netsim_api::{
    chip_error::ChipError,
    chips::{
        BleBeacon, BluetoothMode, BluetoothParams, ChipClient, ChipConfig, ChipRequest,
        DeviceParams, NetworkParams,
    },
    client_error::ClientError,
    device_error::DeviceError,
    devices::{api, CreateDeviceParams, DeviceClient, DeviceConfig},
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
        device_config: DeviceConfig::new(
            "test_device",
            true,
            Default::default(),
            Default::default(),
        ),
        chip_config: create_chip_config(BluetoothMode::Device(DeviceParams {})),
    }
}

fn create_chip_config(mode: BluetoothMode) -> ChipConfig {
    ChipConfig::new(
        "ps_chip",
        "ps_manufacturer",
        "ps_product",
        NetworkParams::Bluetooth(BluetoothParams {
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

fn get_test_create_device_request(device_name: String) -> api::DeviceCreate {
    let chip_create = api::ChipCreate::new(
        "test-bt-chip",
        "Netsim",
        "Netsim BT",
        api::Chip::Beacon(BleBeacon {
            address: "00:11:22:33:44:55".to_string(),
            settings: Default::default(),
            adv_data: Default::default(),
            scan_response: Default::default(),
        }),
    );

    api::DeviceCreate {
        config: DeviceConfig::new(device_name, true, Default::default(), Default::default()),
        chip: chip_create,
    }
}

/// Verifies successful device and chip creation using the CreateDevice API.
#[tokio::test]
async fn test_create_device_succeeds() {
    test_create_device_succeeds_inner().await;
}

async fn test_create_device_succeeds_inner() {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with success.
    tokio::spawn(mock_chip_service_response(chip_rx, Ok(())));

    let device_name = "test-dev-1".to_string();
    let request = get_test_create_device_request(device_name.clone());

    let created_device_id = client.create(Box::new(request)).await.unwrap();

    let list_response = client.list().await.unwrap();
    assert_eq!(list_response.devices.len(), 1);
    let listed_device = list_response
        .devices
        .iter()
        .find(|d| d.id == created_device_id.0)
        .expect("Created device not found in list");

    assert_eq!(listed_device.id, created_device_id.into());
    // TODO: Validate chips after chip info is populated.
}

/// Verifies successful device and chip creation using the PsCreate API.
#[tokio::test]
async fn test_ps_create_success() {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with success.
    tokio::spawn(mock_chip_service_response(chip_rx, Ok(())));

    let result = client.ps_create(create_test_ps_device_params()).await;
    assert!(result.is_ok());
}

/// Ensures proper error handling when chip creation fails during a PsCreate call.
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

/// Checks if the server correctly shuts down after a configured idle period.
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

/// Confirms that devices are correctly listed after creation using the PsCreate API.
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
    let device_config =
        DeviceConfig::new("test_device", true, Default::default(), Default::default());
    assert_eq!(device.name, "test_guid");
    assert_eq!(device.visible, Some(device_config.visible));
    assert_eq!(device.position.as_ref(), Some(&device_config.position));
    assert_eq!(device.orientation.as_ref(), Some(&device_config.orientation));

    // TODO: Verify device.chips once the field is populated.
}
