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
//! - `test_server_shutdown_on_idle`: Checks if the server correctly shuts down after a configured idle period.
//! - `test_list_devices`: Confirms that devices are correctly listed after creation using the PsCreate API.
//! - `test_delete_device_removes_chips`: Verifies that deleting the device also deletes its chips.
//! - `test_delete_nonexistent_device_fails`: Verifies that attempting to delete a non-existent device returns an error.
//! - `test_delete_ps_create_device_fails`: Verifies that attempting to delete a PsCreate device returns an error.
//! - `test_server_shutdown_on_last_chip_delete`: Checks if the server correctly shuts down after the last chip is deleted.
//! - `test_update_device`: Verifies that the Update request modifies device properties.
//! - `test_notify_chip_removed`: Tests the handle_notify_chip_removed logic for the last chip.
//! - `test_notify_chip_removed_leaves_device`: Tests handle_notify_chip_removed when other chips remain.
//! - `test_notify_chip_removed_nonexistent_chip`: Tests handle_notify_chip_removed with a non-existent chip.

use crate::utils::*; // touch
use netsim_model::{
    chip::{ChipId, ChipRequest},
    chip_error::ChipError,
    client_error::ClientError,
    device::{DeviceConfig, DeviceId},
    device_error::DeviceError,
};
use std::time::Duration;
use tokio::sync::oneshot;

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
    assert_eq!(listed_device.name, device_name);
    // TODO: Validate chips after chip info is populated.
}

/// Verifies successful device and chip creation using the PsCreate API.
#[tokio::test]
async fn test_ps_create_success() {
    test_ps_create_success_inner().await;
}
async fn test_ps_create_success_inner() {
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
    assert_eq!(device.name, device_config.name);
    assert_eq!(device.visible, device_config.visible);
    assert_eq!(device.position, device_config.position);
    assert_eq!(device.orientation, device_config.orientation);

    // TODO: Verify device.chips once the field is populated.
}

/// Verifies that deleting the device also deletes its chips.
#[tokio::test]
async fn test_delete_last_chip_removes_device() {
    test_delete_last_chip_removes_device_inner().await;
}
async fn test_delete_last_chip_removes_device_inner() {
    let TestFixture { client, mut chip_rx, .. } = setup();

    // Mock chip creation and deletion
    let (delete_called_tx, delete_called_rx) = tokio::sync::oneshot::channel::<ChipId>();
    tokio::spawn(async move {
        // Handle Create
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
        // Handle Delete
        if let Some(ChipRequest::Delete { id, respond_to }) = chip_rx.recv().await {
            assert_eq!(id, ChipId(0)); // Expecting ChipId 0
            delete_called_tx.send(id).unwrap();
            respond_to.send(Ok(())).unwrap();
        }
    });

    let device_name = "test-dev-del".to_string();
    let request = get_test_create_device_request(device_name.clone());
    let created_device_id = client.create(Box::new(request)).await.unwrap();

    // Verify device exists
    let list_response = client.list().await.unwrap();
    assert_eq!(list_response.devices.len(), 1);

    // Delete the device
    client.delete(created_device_id).await.unwrap();

    // Verify that ChipClient::delete was called on the mock chip service
    match tokio::time::timeout(Duration::from_millis(100), delete_called_rx).await {
        Ok(Ok(chip_id)) => assert_eq!(chip_id, ChipId(0)),
        Err(_) => panic!("Timeout waiting for ChipClient::delete call"),
        Ok(Err(_)) => panic!("Oneshot channel receiver error"),
    }
}

/// Verifies that attempting to delete a non-existent device fails.
#[tokio::test]
async fn test_delete_nonexistent_device_fails() {
    test_delete_nonexistent_device_fails_inner().await;
}

async fn test_delete_nonexistent_device_fails_inner() {
    let TestFixture { client, .. } = setup();

    // Attempt to delete a device ID that hasn't been created
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

/// Verifies that attempting to delete a PsCreate device returns an error.
#[tokio::test]
async fn test_delete_ps_create_device_fails() {
    test_delete_ps_create_device_fails_inner().await.unwrap();
}
async fn test_delete_ps_create_device_fails_inner() -> Result<(), Box<dyn std::error::Error>> {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with success.
    tokio::spawn(mock_chip_service_response(chip_rx, Ok(())));

    // Create a device using PsCreate
    client.ps_create(create_test_ps_device_params()).await?;

    // Get the device ID
    let list_response = client.list().await?;
    assert_eq!(list_response.devices.len(), 1);
    let device_id = DeviceId(list_response.devices[0].id);

    // Attempt to delete the device
    let result = client.delete(device_id).await;

    assert!(
        matches!(result, Err(ClientError::Device(ref e)) if matches!(**e, DeviceError::InvalidArguments(_)))
    );
    if let Err(ClientError::Device(e)) = result {
        if let DeviceError::InvalidArguments(msg) = *e {
            assert!(msg.contains("is a PsCreate device and cannot be deleted directly"));
        } else {
            panic!("Unexpected error type: {:?}", e);
        }
    } else {
        panic!("Expected an error, got {:?}", result);
    }
    Ok(())
}

/// Checks if the server correctly shuts down after the last chip is deleted.
#[tokio::test]
async fn test_server_shutdown_on_last_chip_delete() {
    test_server_shutdown_on_last_chip_delete_inner().await.unwrap();
}
async fn test_server_shutdown_on_last_chip_delete_inner() -> Result<(), Box<dyn std::error::Error>>
{
    let TestFixture { client, server_task, mut chip_rx, .. } = setup_for_idle_test();
    let (delete_called_tx, delete_called_rx) = oneshot::channel::<ChipId>();

    tokio::spawn(async move {
        // Handle Create
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
        // Handle Delete
        if let Some(ChipRequest::Delete { id, respond_to }) = chip_rx.recv().await {
            delete_called_tx.send(id).unwrap();
            respond_to.send(Ok(())).unwrap();
        }
    });

    let device_name = "test-dev-shutdown".to_string();
    let request = get_test_create_device_request(device_name.clone());
    let device_id = client.create(Box::new(request)).await?;

    // Delete the device
    client.delete(device_id).await?;

    // Verify that ChipClient::delete was called on the mock chip service
    match tokio::time::timeout(Duration::from_millis(100), delete_called_rx).await {
        Ok(Ok(chip_id)) => assert_eq!(chip_id, ChipId(0)),
        Err(_) => panic!("Timeout waiting for ChipClient::delete call"),
        Ok(Err(_)) => panic!("Oneshot channel receiver error"),
    }

    // Simulate the chip service notifying the device service of chip removal
    client.notify_chip_removed_block(ChipId(0)).await?;
    // Wait for longer than the idle timeout for the server to shut down.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(server_task.is_finished());

    // Any subsequent command should fail because the server is down.
    let result = client.list().await;
    assert!(matches!(result, Err(ClientError::Send(_))));
    Ok(())
}

/// Verifies that the Update request modifies device properties.
#[tokio::test]
async fn test_update_device() {
    // TODO: Make a ChipClient mock that handles different return types
    //    test_update_device_inner().await;
}

/// Tests the handle_notify_chip_removed logic for the last chip, which should remove the device.
#[tokio::test]
async fn test_notify_chip_removed() {
    test_notify_chip_removed_inner().await.unwrap();
}
async fn test_notify_chip_removed_inner() -> Result<(), Box<dyn std::error::Error>> {
    let TestFixture { client, chip_rx, .. } = setup();

    // Mock the chip service to respond with success.
    tokio::spawn(mock_chip_service_response(chip_rx, Ok(())));

    // Create a device
    let _device_id =
        client.create(Box::new(get_test_create_device_request("test-dev".to_string()))).await?;
    let list_response = client.list().await?;
    assert_eq!(list_response.devices.len(), 1);

    // Simulate NotifyChipRemoved and wait for processing.
    client.notify_chip_removed_block(ChipId(0)).await?;

    // Verify device is removed
    let list_response_after_notify = client.list().await?;
    assert_eq!(list_response_after_notify.devices.len(), 0);
    Ok(())
}

/// Tests handle_notify_chip_removed with a non-existent chip.
#[tokio::test]
async fn test_notify_chip_removed_nonexistent_chip() {
    test_notify_chip_removed_nonexistent_chip_inner().await.unwrap();
}
async fn test_notify_chip_removed_nonexistent_chip_inner() -> Result<(), Box<dyn std::error::Error>>
{
    let TestFixture { client, .. } = setup();

    // Simulate NotifyChipRemoved for a chip that doesn't exist
    client.notify_chip_removed_block(ChipId(999)).await.unwrap();

    // Allow some time for the spawned task to potentially run and log errors
    tokio::time::sleep(Duration::from_millis(10)).await;

    // No panic should occur, and the server should still be running.
    let list_response = client.list().await?;
    assert_eq!(list_response.devices.len(), 0);
    Ok(())
}
