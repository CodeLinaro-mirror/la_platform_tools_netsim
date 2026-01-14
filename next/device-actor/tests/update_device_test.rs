// Copyright (C) 2025 The Android Open Source Project

// Feature: Update Device
//
//   As a user
//   I want to update device properties
//   So that I can simulate dynamic changes in the environment
//
//   Scenario: Update device properties propagates to chips
//     Given a running Device Actor
//     And the Link Client expecting a NotifyChipAdded event
//     And the Chip Client expecting Create and Update calls
//     When I create a device and update its properties
//     Then the device properties are updated
//     And the Chip Client received the expected Update call
//
//   Scenario: Notify chip removed
//     Given a running Device Actor
//     And the Link Client expecting NotifyChipAdded and NotifyChipRemoved
//     And the Chip Client expecting a Create call
//     When I create a device and notify that a chip was removed
//     Then the device no longer contains the chip

use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate, DeviceUpdate};
use device_api::DeviceConfig;
use netsim_model::chip::{BleBeacon, ChipId, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

#[tokio::test]
async fn test_update_device_propagates_to_chips() {
    // Given the Link Client expecting a NotifyChipAdded event
    let (mut mock_link_controller, mock_link_client) = link_api::mock::MockLinkClient::new();
    // Expect NotifyChipAdded
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());

    // And the Chip Client expecting Create and Update calls
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| Ok(()));
    mock_chip_client
        .expect_update()
        .withf(|_, patch| patch.position.is_some() && patch.orientation.is_some())
        .times(1)
        .returning(|_, _| Ok(netsim_model::chip::Chip::default()));

    // And a running Device Actor
    let mut chip_clients: HashMap<NetworkKind, Box<dyn netsim_model::chip::ChipClient>> =
        HashMap::new();
    chip_clients.insert(NetworkKind::Bluetooth, Box::new(mock_chip_client));

    let (runner, client) = device_actor::new();
    let actor = DeviceActor::new(
        chip_clients,
        Arc::new(AtomicU32::new(0)),
        None,
        Box::new(mock_link_client),
    );
    tokio::spawn(runner.run(actor));

    // When I create a device and update its properties
    let params = DeviceCreate {
        device_config: DeviceConfig::new(
            "test-dev".to_string(),
            true,
            Default::default(),
            Default::default(),
        ),
        chip: DeviceChipCreate {
            name: "beacon".to_string(),
            manufacturer: "Netsim".to_string(),
            product_name: "NetsimBeacon".to_string(),
            chip: Chip::Beacon(BleBeacon::default()),
        },
    };
    let device_id = client.create_device(params).await.unwrap();

    let mut update = DeviceUpdate::default();
    update.id = device_id.0; // DeviceUpdate needs ID
    update.position = Some(device_api::Position { x: 10.0, y: 10.0, z: 0.0 });
    update.orientation = Some(device_api::Orientation { yaw: 1.0, pitch: 0.0, roll: 0.0 });
    update.name = Some("updated-name".to_string());

    client.update(device_id, update).await.unwrap();

    // Then the device properties are updated
    let device = client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.position.x, 10.0);
    assert_eq!(device.orientation.yaw, 1.0);
    assert_eq!(device.name, "updated-name");

    // And the Chip Client received the expected Update call (verified by checkpoint)
}

#[tokio::test]
async fn test_notify_chip_removed() {
    // Given the Link Client expecting NotifyChipAdded and NotifyChipRemoved
    let (mut mock_link_controller, mock_link_client) = link_api::mock::MockLinkClient::new();
    // Expect NotifyChipAdded
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());
    // Expect NotifyChipRemoved
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());

    // And the Chip Client expecting a Create call
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| Ok(()));

    // And a running Device Actor
    let mut chip_clients: HashMap<NetworkKind, Box<dyn netsim_model::chip::ChipClient>> =
        HashMap::new();
    chip_clients.insert(NetworkKind::Bluetooth, Box::new(mock_chip_client));

    let (runner, client) = device_actor::new();
    let actor = DeviceActor::new(
        chip_clients,
        Arc::new(AtomicU32::new(0)),
        None,
        Box::new(mock_link_client),
    );
    tokio::spawn(runner.run(actor));

    // When I create a device and notify that a chip was removed
    let params = DeviceCreate {
        device_config: DeviceConfig::new(
            "test-dev".to_string(),
            true,
            Default::default(),
            Default::default(),
        ),
        chip: DeviceChipCreate {
            name: "beacon".to_string(),
            manufacturer: "Netsim".to_string(),
            product_name: "NetsimBeacon".to_string(),
            chip: Chip::Beacon(BleBeacon::default()),
        },
    };

    let device_id = client.create_device(params).await.unwrap();
    let device = client.get(device_id).await.unwrap().unwrap();
    let chip_id = ChipId(device.chips[0].id);

    client.notify_chip_removed(device_id, chip_id).await.unwrap(); // Using blocking version via wrapper or similar?

    // Then the device no longer contains the chip
    let device = client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.chips.len(), 0);

    // Explicit verification happens at drop
}
