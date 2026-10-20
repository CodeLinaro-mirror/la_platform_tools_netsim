// Copyright (C) 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// Feature: Device Error Handling
//
//   As a user
//   I want the system to handle invalid operations gracefully
//   So that my application doesn't crash on bad input

use device_api::{DeviceId, DeviceUpdate};

use crate::world::World;

// Scenario: Update a non-existent device
//   Given a running Device Actor
//   When I attempt to update a device that does not exist
//   Then the operation fails with a DeviceNotFound error
#[tokio::test]
async fn test_update_non_existent_device_fails() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I attempt to update a device that does not exist
    let non_existent_id = DeviceId(9999);
    let update = DeviceUpdate::default();

    let result = world.client.update(non_existent_id, update).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Device not found"));
}

// Scenario: Delete a non-existent device
//   Given a running Device Actor
//   When I attempt to delete a device that does not exist
//   Then the operation fails with a DeviceNotFound error
#[tokio::test]
async fn test_delete_non_existent_device_fails() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I attempt to delete a device that does not exist
    let non_existent_id = DeviceId(9999);
    let result = world.client.delete(non_existent_id).await;

    // Then the operation fails with a DeviceNotFound error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Device not found"));
}
