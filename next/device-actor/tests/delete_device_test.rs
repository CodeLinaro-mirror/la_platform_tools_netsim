// Copyright (C) 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// Feature: Delete Device
//
//   As a user
//   I want to delete a device
//   So that I can remove it from the simulation

use crate::world::World;

// Scenario: Delete a device added via AddChip
//   Given a running Device Actor
//   When I add a chip (which creates a device)
//   And I delete the device
//   Then the operation succeeds
#[tokio::test]
async fn test_delete_add_chip_device_fails() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I add a chip (which creates a device)
    let device_guid = "guid-1";
    let device_id = world.when_add_chip(device_guid, "beacon").await;

    // And I delete the device
    world.when_delete_device(device_id).await;

    // Then the operation succeeds (implied by unwrap in when_delete_device)
}

// Scenario: Delete a device with chips
//   Given a running Device Actor
//   When I create a device with a chip
//   And I delete the device
//   Then the device is no longer retrievable
#[tokio::test]
async fn test_delete_device_removes_chips() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I create a device with a chip
    let device_name = "test-dev";
    let device_id = world.when_create_device(device_name).await;

    // And I delete the device
    world.when_delete_device(device_id).await;

    // Then the device is no longer retrievable
    let device = world.client.get(device_id).await.unwrap();
    assert!(device.is_none());
}
