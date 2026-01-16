// Copyright (C) 2025 The Android Open Source Project

// Feature: Update Device
//
//   As a user
//   I want to update device properties
//   So that I can simulate dynamic changes in the environment

use crate::world::World;
use device_api::api::DeviceUpdate;
use netsim_model::chip::{ChipId, NetworkKind};
use std::collections::HashMap;

// Scenario: Update device properties propagates to chips
//   Given a running Device Actor with expectation for Update call
//   When I create a device and update its properties
//   Then the device properties are updated
//   And the Chip Client received the expected Update call
#[tokio::test]
async fn test_update_device_propagates_to_chips() {
    // Given a running Device Actor with expectation for Update call
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| Ok(()));
    mock_chip_client
        .expect_update()
        .withf(|_, patch| patch.position.is_some() && patch.orientation.is_some())
        .times(1)
        .returning(|_, _| Ok(netsim_model::chip::Chip::default()));

    let mut chip_clients = HashMap::new();
    chip_clients.insert(
        NetworkKind::Bluetooth,
        Box::new(mock_chip_client) as Box<dyn netsim_model::chip::ChipClient>,
    );

    // Use default link mock
    let mut mock_link_client = link_api::MockLinkClient::new();
    mock_link_client.expect_action().returning(|_, _| Ok(()));
    mock_link_client.expect_create().returning(|_| Ok(link_api::LinkId(0)));

    let world = World::with_clients(chip_clients, mock_link_client).await;

    // When I create a device and update its properties
    let device_name = "test-dev";
    let device_id = world.when_create_device(device_name).await;

    let mut update = DeviceUpdate::default();
    update.id = device_id.0;
    update.position = Some(device_api::Position { x: 10.0, y: 10.0, z: 0.0 });
    update.orientation = Some(device_api::Orientation { yaw: 1.0, pitch: 0.0, roll: 0.0 });
    update.name = Some("updated-name".to_string());

    world.when_update_device(device_id, update).await;

    // Then the device properties are updated
    let device = world.client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.position.x, 10.0);
    assert_eq!(device.orientation.yaw, 1.0);
    assert_eq!(device.name, "updated-name");

    // And the Chip Client received the expected Update call (verified by checkpoint)
}

// Scenario: Notify chip removed
//   Given a running Device Actor
//   When I create a device and notify that a chip was removed
//   Then the device no longer contains the chip
#[tokio::test]
async fn test_notify_chip_removed() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I create a device and notify that a chip was removed
    let device_name = "test-dev";
    let device_id = world.when_create_device(device_name).await;
    let device = world.client.get(device_id).await.unwrap().unwrap();
    let chip_id = ChipId(device.chips[0].id);

    world.when_notify_chip_removed(device_id, chip_id).await;

    // Then the device no longer contains the chip
    let device = world.client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.chips.len(), 0);
}
