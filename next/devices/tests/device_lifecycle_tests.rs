// Copyright 2025 The Android Open Source Project

//! Tests for device lifecycle management.
//!
//! This module contains unit tests for the device server's lifecycle operations,
//! including device creation, deletion, chip creation success/failure, server shutdown on idle,
//! and listing devices.
//!
//! List of tests:
//! - `test_create_device_succeeds`: Verifies successful device and chip creation using the CreateDevice API.
//! - `test_ps_create_success`: Verifies successful device and chip creation using the PsCreate API.
//! - `test_ps_create_chip_failure`: Ensures proper error handling when chip creation fails during a PsCreate call.
//! - `test_delete_last_chip_removes_device`: Verifies that deleting the last chip on a device removes the device.
//! - `test_delete_nonexistent_chip_fails`: Verifies that attempting to delete a non-existent chip returns an error.
//! - `test_server_shutdown_on_idle`: Checks if the server correctly shuts down after a configured idle period.
//! - `test_server_shutdown_on_last_chip_delete`: Checks if the server correctly shuts down after the last chip is deleted.
//! - `test_list_devices`: Confirms that devices are correctly listed after creation using the PsCreate API.

use crate::utils::*;
use netsim_api::{
    chip_error::ChipError,
    chips::{ChipId, ChipRequest},
    client_error::ClientError,
    device_error::DeviceError,
    devices::{DeviceConfig, DeviceId},
};
use std::time::Duration;

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
    test_ps_create_chip_failure_inner().await;
}
async fn test_ps_create_chip_failure_inner() {
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
    test_server_shutdown_on_idle_inner().await;
}
async fn test_server_shutdown_on_idle_inner() {
    let TestFixture { client, server_task, .. } = setup_for_idle_test();

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

/// Verifies that deleting the last chip removes the device.
#[tokio::test]
async fn test_delete_last_chip_removes_device() {
    test_delete_last_chip_removes_device_inner().await;
}
async fn test_delete_last_chip_removes_device_inner() {
    let TestFixture { client, mut chip_rx, .. } = setup();

    // Mock chip creation
    tokio::spawn(async move {
        // Handle Create
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
        // Handle Delete
        if let Some(ChipRequest::Delete { id, respond_to }) = chip_rx.recv().await {
            assert_eq!(id, ChipId(0)); // Expecting ChipId 0
            respond_to.send(Ok(())).unwrap();
        }
    });

    let device_name = "test-dev-del".to_string();
    let request = get_test_create_device_request(device_name.clone());
    let created_device_id = client.create(Box::new(request)).await.unwrap();

    // Verify device exists
    let list_response = client.list().await.unwrap();
    assert_eq!(list_response.devices.len(), 1);

    // Delete the chip
    client.delete(created_device_id).await.unwrap();

    // Verify device is removed
    let list_response_after_delete = client.list().await.unwrap();
    assert_eq!(list_response_after_delete.devices.len(), 0);
}

/// Verifies that attempting to delete a non-existent chip fails.
#[tokio::test]
async fn test_delete_nonexistent_device_fails() {
    test_delete_nonexistent_device_fails_inner().await;
}

async fn test_delete_nonexistent_device_fails_inner() {
    let TestFixture { client, .. } = setup();

    // Attempt to delete a chip ID that hasn't been created
    let result = client.delete(DeviceId(9999)).await;

    assert!(
        matches!(result, Err(ClientError::Device(ref e)) if matches!(**e, DeviceError::InvalidArguments(_)))
    );
    if let Err(ClientError::Device(e)) = result {
        if let DeviceError::InvalidArguments(msg) = *e {
            assert!(msg.contains("Device 9999 not found"));
        } else {
            panic!("Unexpected error type");
        }
    } else {
        panic!("Expected an error");
    }
}

/// Checks if the server correctly shuts down after the last chip is deleted.
#[tokio::test]
async fn test_server_shutdown_on_last_chip_delete() {
    test_server_shutdown_on_last_chip_delete_inner().await;
}
async fn test_server_shutdown_on_last_chip_delete_inner() {
    // We need a separate setup to control the chip_rx channel for mocking delete
    let TestFixture { client, server_task, mut chip_rx } = setup_for_idle_test();

    tokio::spawn(async move {
        // Handle Create
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
        // Handle Delete
        if let Some(ChipRequest::Delete { id, respond_to }) = chip_rx.recv().await {
            assert_eq!(id, ChipId(0)); // Expecting ChipId 0
            respond_to.send(Ok(())).unwrap();
        }
    });

    let device_name = "test-dev-shutdown".to_string();
    let request = get_test_create_device_request(device_name.clone());
    let device_id = client.create(Box::new(request)).await.unwrap();

    // Delete the chip
    client.delete(device_id).await.unwrap();

    // Wait for longer than the timeout for the server to shut down.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(server_task.is_finished());

    // Any subsequent command should fail because the server is down.
    let result = client.list().await;
    assert!(matches!(result, Err(ClientError::Send(_))));
}
