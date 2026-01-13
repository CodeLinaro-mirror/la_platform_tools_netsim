// Copyright (C) 2025 The Android Open Source Project

// Feature: Add Chip to Device
//
//   As a device owner
//   I want to add simulated chips to my device
//   So that I can emulate various network capabilities
//
//   Scenario: Successfully add a chip to an existing device
//     Given the Link Client expecting a NotifyChipAdded event
//     And the Chip Client expecting a Create call
//     And a running Device Actor
//     When I request to add a new "Bluetooth" chip to a device
//     Then the operation succeeds
//     And the device can be retrieved and contains the new chip
//
//   Scenario: Fail to add a chip when chip creation fails
//     Given the Link Client expecting NO action
//     And the Chip Client configured to fail on Create
//     And a running Device Actor
//     When I request to add a new chip
//     Then the operation fails
//     And the device is not created (Get returns None)

use device_actor::DeviceActor;
use device_api::DeviceAddChip;
use device_api::DeviceConfig;
use netsim_model::chip::NetworkKind;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

#[tokio::test]
async fn test_add_chip_success() {
    // Given a mock Link Client expecting a NotifyChipAdded event
    let mut mock_link_client = link_api::MockLinkClient::new();
    mock_link_client
        .expect_action()
        .withf(|_, action| matches!(action, link_api::LinkAction::NotifyChipAdded(_, _)))
        .returning(|_, _| Ok(())); // Correct signature for action result

    // And a mock Chip Client expecting a Create call
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| Ok(()));

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

    // When I request to add a new "Bluetooth" chip to a device
    let params = DeviceAddChip {
        device_guid: "guid-1".to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig::new(
            "test-dev".to_string(),
            true,
            Default::default(),
            Default::default(),
        ),
        chip_config: netsim_model::chip::ChipConfig {
            name: "chip-1".to_string(),
            manufacturer: "man-1".to_string(),
            product_name: "prod-1".to_string(),
            network_params: netsim_model::chip::NetworkParams::Bluetooth(
                netsim_model::chip::BluetoothCreate {
                    address: "00:00:00:00:00:00".to_string(),
                    bt_properties: Default::default(),
                    mode: netsim_model::chip::BluetoothMode::Device(Default::default()),
                },
            ),
        },
    };
    let result = client.add_chip(params).await;

    // Then the operation should succeed
    assert!(result.is_ok(), "AddChip should succeed");
    let device_id = result.unwrap();

    // And the device can be retrieved and contains the new chip
    let device =
        client.get(device_id).await.expect("Failed to get device").expect("Device not found");
    assert_eq!(device.chips.len(), 1);
    assert_eq!(device.chips[0].name, Some("chip-1".to_string()));
}

#[tokio::test]
async fn test_add_chip_chip_failure() {
    // Given a mock Link Client expecting NO action
    let mock_link_client = link_api::MockLinkClient::new();
    // No expectations pushed = expects 0 calls

    // And a mock Chip Client that fails to create a chip
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| {
        Err(netsim_model::client_error::ClientError::Send("Simulated Error".into()))
    });

    // And a Device Actor initialized with these mocks
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

    // When I request to add a new chip
    let params = DeviceAddChip {
        device_guid: "guid-2".to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig::new(
            "test-dev".to_string(),
            true,
            Default::default(),
            Default::default(),
        ),
        chip_config: netsim_model::chip::ChipConfig {
            name: "chip-2".to_string(),
            manufacturer: "man-1".to_string(),
            product_name: "prod-1".to_string(),
            network_params: netsim_model::chip::NetworkParams::Bluetooth(
                netsim_model::chip::BluetoothCreate {
                    address: "00:00:00:00:00:00".to_string(),
                    bt_properties: Default::default(),
                    mode: netsim_model::chip::BluetoothMode::Device(Default::default()),
                },
            ),
        },
    };
    let result = client.add_chip(params).await;

    // Then the operation should fail
    assert!(result.is_err(), "AddChip should fail");

    // And the device is not created (Get returns None)
    let list = client.list().await.expect("List failed");
    assert_eq!(list.devices.len(), 0, "No devices should exist");

    // Check probable ID 0 just to cover "Get(id) fails" case
    let device = client.get(device_api::DeviceId(0)).await.expect("Get failed");
    assert!(device.is_none(), "Device 0 should not exist");
}
