// Copyright (C) 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// Feature: Create Device
//
//   As a user
//   I want to create a device
//   So that I can simulate its behavior

use crate::world::World;

// Scenario: Successfully create a device
//   Given a running Device Actor
//   When I create a new device
//   Then the device creation succeeds
//   And the returned device properties match the configuration
#[tokio::test]
async fn test_create_device_succeeds() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I create a new device
    let device_name = "test-dev-1";
    let device_id = world.when_create_device(device_name).await;

    // Then the device creation succeeds
    // (implied by awaiting match in when_create_device)

    // And the returned device properties match the configuration
    let device = world.client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.name, device_name);
    assert_eq!(device.chips.len(), 1);
}
