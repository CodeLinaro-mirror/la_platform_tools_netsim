// Copyright (C) 2025 The Android Open Source Project

// Feature: Reset Simulation
//
//   As a developer or person testing the system
//   I want to reset the simulation state
//   So that I can return to a known baseline for further testing

use crate::world::World;

// Scenario: Global reset clears all active links and resets all devices
//   Given a running Device Actor
//   And multiple devices are created
//   And these devices have modified properties (visibility, position)
//   When I call the global reset RPC
//   Then all devices should have their properties reset to defaults
//   And the link client reset should have been called
#[tokio::test]
async fn test_global_reset_behavior() {
    let world = World::new().await;

    // Given multiple devices are created
    let id1 = world.when_add_chip("guid-1", "chip-1").await;
    let id2 = world.when_add_chip("guid-2", "chip-2").await;

    // And these devices have modified properties
    world.given_all_devices_are_modified().await;

    // When I call the global reset RPC
    world.when_reset_is_called().await;

    // Then all devices should have their properties reset to defaults
    world.then_all_devices_are_reset().await;

    // And the link client reset should have been called
    world.then_link_reset_was_called();
}

// Scenario: Individual device reset only affects target device
//   Given a running Device Actor with two devices
//   And both devices have modified properties
//   When I call the reset RPC for the first device
//   Then the first device should have its properties reset
//   And the second device should still have its modified properties
//   And the link client reset should NOT have been called
#[tokio::test]
async fn test_individual_device_reset_behavior() {
    let world = World::new().await;

    // Given a running Device Actor with two devices
    let id1 = world.when_add_chip("guid-1", "chip-1").await;
    let id2 = world.when_add_chip("guid-2", "chip-2").await;

    // And both devices have modified properties
    world.given_device_is_modified(id1).await;
    world.given_device_is_modified(id2).await;

    // When I call the reset RPC for the first device
    world.when_reset_device_is_called(id1).await;

    // Then the first device should have its properties reset
    world.then_device_properties_are_reset(id1).await;

    // And the second device should still have its modified properties
    let device2 = world.client.get(id2).await.unwrap().unwrap();
    assert!(!device2.visible, "Device 2 should STILL be invisible");
    assert_ne!(
        device2.position,
        device_api::Position::default(),
        "Device 2 position should STILL be modified"
    );

    // And the link client reset should NOT have been called
    world.then_link_reset_was_not_called();
}
