// Copyright (C) 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

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
    world.when_device_is_modified(id1).await;
    world.when_device_is_modified(id2).await;

    // When I call the reset RPC for the first device
    world.when_reset_device_is_called(id1).await;

    // Then the first device should have its properties reset
    world.then_device_properties_are_reset(id1).await;

    // And the second device should still have its modified properties
    let device2 = world.client.get(id2).await.unwrap().unwrap();
    assert!(!device2.visible, "Device 2 should STILL be invisible");
    assert_ne!(
        device2.pose.position,
        device_api::Position::default(),
        "Device 2 position should STILL be modified"
    );

    // And the link client reset should NOT have been called
    world.then_link_reset_was_not_called();
}

// Scenario: Reset should re-enable all chips
//   Given a running Device Actor with a chip
//   When the chip is disabled
//   And I call the global reset RPC
//   Then the chip should be re-enabled
#[tokio::test]
async fn test_reset_re_enables_chips() {
    let world = World::new().await;

    // Given a running Device Actor with a chip
    let id1 = world.when_add_chip("guid-1", "chip-1").await;

    // Disable the Bluetooth radios (le_state and classic_state)
    let chip_update = World::create_bluetooth_chip_update(Some(false), Some(false));
    world.when_update_device_chip(id1, chip_update).await;

    // Verify it is disabled
    world.then_bluetooth_states_are(id1, false, false).await;

    // When I call the global reset RPC
    world.when_reset_is_called().await;

    // Then the chip radios should be re-enabled!
    world.then_bluetooth_states_are(id1, true, true).await;
}

// Scenario: Reset restores initial non-default position and orientation
//   Given a device created with non-default position and orientation
//   And the device's properties are modified
//   When I call the reset RPC
//   Then the device should have its properties reset to the INITIAL values
#[tokio::test]
async fn test_reset_restores_initial_non_default_properties() {
    let world = World::new().await;

    // Given a device created with non-default position and orientation
    let initial_pos = device_api::Position { x: 10.0, y: 20.0, z: 30.0 };
    let initial_orient = device_api::Orientation { yaw: 1.0, pitch: 2.0, roll: 3.0 };
    let id = world
        .when_create_device_at_position_and_orientation(
            "test-device",
            initial_pos.clone(),
            initial_orient.clone(),
        )
        .await;

    // And the device's properties are modified
    world.when_device_is_modified(id).await;

    // When I call the reset RPC
    world.when_reset_device_is_called(id).await;

    // Then the device should have its properties reset to the INITIAL values
    world
        .then_device_and_chips_position_and_orientation_match(id, initial_pos, initial_orient)
        .await;
}

// Scenario: Reset propagates to all chip actors for a device
//   Given a device with multiple chips created with non-default position and
// orientation   And the device's properties are modified
//   When I call the reset RPC for the device
//   Then all chip actors for that device should have their properties reset
#[tokio::test]
async fn test_reset_propagates_to_all_chip_actors() {
    let world = World::new().await;

    // Given a device with multiple chips created with non-default position and
    // orientation
    let initial_pos = device_api::Position { x: 10.0, y: 20.0, z: 30.0 };
    let initial_orient = device_api::Orientation { yaw: 1.0, pitch: 2.0, roll: 3.0 };

    // Create first chip (creates the device)
    let device_id = world
        .when_create_device_at_position_and_orientation(
            "multi-chip-device",
            initial_pos.clone(),
            initial_orient.clone(),
        )
        .await;

    // Add second chip (WiFi) to the same device
    let mut add_chip_params = World::create_device_add_chip_params(
        "multi-chip-device-guid".to_string(),
        "wifi-chip".to_string(),
        "".to_string(),
    );
    add_chip_params.device_config.pose.position = initial_pos.clone();
    add_chip_params.device_config.pose.orientation = initial_orient.clone();
    add_chip_params.chip_config.chip_kind_params =
        netsim_model::ChipKindParams::Wifi(Default::default());
    world.client.add_chip(add_chip_params).await.unwrap();

    // And the device's properties are modified
    world.when_device_is_modified(device_id).await;

    // When I call the reset RPC
    world.when_reset_device_is_called(device_id).await;

    // Then all chip actors should have their properties reset to the INITIAL values
    world
        .then_device_and_chips_position_and_orientation_match(
            device_id,
            initial_pos.clone(),
            initial_orient.clone(),
        )
        .await;

    // Verify via ChipActors (Mocks) directly
    world
        .then_all_chip_actors_position_and_orientation_match(device_id, initial_pos, initial_orient)
        .await;
}
