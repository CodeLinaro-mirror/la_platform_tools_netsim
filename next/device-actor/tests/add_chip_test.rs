// Copyright (C) 2025 The Android Open Source Project

// Feature: Add Chip Integration
//
//   As a user
//   I want to add chips to devices via PacketStream
//   So that I can support multiple chips per device and dynamic attachment

use crate::world::World;
use netsim_model::chip::{MockChipClient, NetworkKind};
use std::collections::HashMap;

// Scenario: Add chip to new device
//   Given a running Device Actor
//   When I add a chip
//   Then a new device is created
//   And the device contains the chip
#[tokio::test]
async fn test_add_chip_creates_new_device() {
    // Given a running Device Actor
    let world = World::new().await;

    // When I add a chip
    let device_guid = "guid-1";
    let device_id = world.when_add_chip(device_guid, "beacon").await;

    // Then a new device is created (implied by ID return)

    // And the device contains the chip
    let device = world.client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.chips.len(), 1);
    assert_eq!(device.chips[0].name, Some("beacon".to_string()));
}

// Scenario: Add chip to existing device
//   Given a running Device Actor
//   When I add a chip
//   And I add another chip with the same device GUID
//   Then the device contains both chips
#[tokio::test]
async fn test_add_chip_to_existing_device() {
    // Given a running Device Actor with expectation for multiple Create calls on ChipClient
    let mut mock_chip_client = MockChipClient::new();
    mock_chip_client.expect_create().times(2).returning(|_| Ok(()));

    let mut chip_clients = HashMap::new();
    chip_clients.insert(
        NetworkKind::Bluetooth,
        Box::new(mock_chip_client) as Box<dyn netsim_model::chip::ChipClient>,
    );

    let mut mock_link_client = link_api::MockLinkClient::new();
    mock_link_client.expect_action().returning(|_, _| Ok(()));
    mock_link_client.expect_create().returning(|_| Ok(link_api::LinkId(0)));
    mock_link_client.expect_notify_chip_added().returning(|_, _| Ok(()));

    let world = World::with_clients(chip_clients, mock_link_client).await;

    // When I add a chip
    let device_guid = "guid-shared";
    let device_id_1 = world.when_add_chip(device_guid, "beacon-1").await;

    // And I add another chip with the same device GUID
    let device_id_2 = world.when_add_chip(device_guid, "beacon-2").await;

    // Then the device contains both chips and IDs match
    assert_eq!(device_id_1, device_id_2);

    let device = world.client.get(device_id_1).await.unwrap().unwrap();
    assert_eq!(device.chips.len(), 2);

    // Verify names
    let names: Vec<String> = device.chips.iter().filter_map(|c| c.name.clone()).collect();
    assert!(names.contains(&"beacon-1".to_string()));
    assert!(names.contains(&"beacon-2".to_string()));
}
