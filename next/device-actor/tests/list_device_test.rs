// Copyright (C) 2025 The Android Open Source Project

// Feature: List Devices
//
//   As a user
//   I want to list all created devices
//   So that I can see the current state of the simulation

use crate::world::World;

// Scenario: List devices when no devices exist
//   Given a running Device Actor
//   When I list devices
//   Then the list is empty
#[tokio::test]
async fn test_list_devices_empty() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I list devices
    let response = world.client.list().await.unwrap();

    // Then the list is empty
    assert!(response.devices.is_empty());
}

// Scenario: List devices with created devices
//   Given a running Device Actor
//   When I create multiple devices
//   And I list devices
//   Then the list contains all created devices
#[tokio::test]
async fn test_list_devices_populated() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I create multiple devices
    let device_names = vec!["dev-1", "dev-2", "dev-3"];
    let mut created_ids = Vec::new();

    for name in &device_names {
        created_ids.push(world.when_create_device(name).await);
    }

    // And I list devices
    let response = world.client.list().await.unwrap();

    // Then the list contains all created devices
    assert_eq!(response.devices.len(), 3);

    for id in created_ids {
        assert!(response.devices.iter().any(|d| d.id == id.0));
    }
}
