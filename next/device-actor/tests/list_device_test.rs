// Copyright (C) 2025 The Android Open Source Project

// Feature: List Devices
//
//   As a user
//   I want to list all devices
//   So that I can verify the current state of the simulation
//
//   Scenario: Configure and list devices
//     Given a running Device Actor
//     And the Link Client expecting a NotifyChipAdded event
//     And the Chip Client expecting a Create call
//     When I create a new device
//     And I request a list of all devices
//     Then the list contains exactly one device
//     And the device properties match the configuration

use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};
use device_api::DeviceConfig;
use netsim_model::chip::{BleBeacon, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

#[tokio::test]
async fn test_list_devices_explicit() {
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
        Box::new(mock_link_client),
    );
    tokio::spawn(runner.run(actor));

    // When I create a new device
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
    let _device_id = client.create_device(params).await.unwrap();

    // And I request a list of all devices
    let list_response = client.list().await.unwrap();

    // Then the list contains exactly one device
    assert_eq!(list_response.devices.len(), 1);
    let device = &list_response.devices[0];

    // And the device properties match the configuration
    assert_eq!(device.name, "test-dev");
    assert_eq!(device.visible, true);

    // Explicit verification happens at drop
}
