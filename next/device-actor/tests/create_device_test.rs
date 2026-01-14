// Copyright (C) 2025 The Android Open Source Project

// Feature: Create Device
//
//   As a user
//   I want to create a device
//   So that I can simulate its behavior
//
//   Scenario: Successfully create a device
//     Given the Link Client expecting a NotifyChipAdded event
//     And the Chip Client expecting a Create call
//     And a running Device Actor
//     When I create a new device
//     Then the device creation succeeds
//     And the returned device properties match the configuration

use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};
use device_api::DeviceConfig;
use netsim_model::chip::{BleBeacon, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

#[tokio::test]
async fn test_create_device_succeeds() {
    // Given the Link Client expecting a NotifyChipAdded event
    let (mut mock_link_controller, mock_link_client) = link_api::mock::MockLinkClient::new();
    // Expect NotifyChipAdded
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
        Box::new(mock_link_client.clone()),
    );
    tokio::spawn(runner.run(actor));

    // When I create a new device
    let device_name = "test-dev-1".to_string();
    let params = DeviceCreate {
        device_config: DeviceConfig::new(
            device_name.clone(),
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

    // Then the device creation succeeds
    // (implied by unwrap above)

    // And the returned device properties match the configuration
    assert_eq!(device.name, device_name);
    assert_eq!(device.chips.len(), 1);

    // Explicit verification happens at drop
}
