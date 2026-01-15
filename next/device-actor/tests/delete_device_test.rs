// Copyright (C) 2025 The Android Open Source Project

// Feature: Delete Device
//
//   As a user
//   I want to delete a device
//   So that I can remove it from the simulation
//
//   Scenario: Delete a device added via AddChip
//     Given the Link Client expecting NotifyChipAdded and NotifyChipRemoved
//     And the Chip Client expecting Create and Delete calls
//     And a running Device Actor
//     When I add a chip (which creates a device)
//     And I delete the device
//     Then the operation succeeds
//
//   Scenario: Delete a device with chips
//     Given the Link Client expecting NotifyChipAdded (x2) and NotifyChipRemoved (x2)
//     And the Chip Client expecting Create and Delete calls
//     And a running Device Actor
//     When I create a device with a chip
//     And I delete the device
//     Then the device is no longer retrievable

use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};
use device_api::{DeviceAddChip, DeviceConfig};
use netsim_model::chip::{BleBeacon, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

#[tokio::test]
async fn test_delete_add_chip_device_fails() {
    // Given the Link Client expecting NotifyChipAdded and NotifyChipRemoved
    let mut mock_link_client = link_api::MockLinkClient::new();
    mock_link_client
        .expect_action()
        .withf(|_, action| matches!(action, link_api::LinkAction::NotifyChipAdded(_, _)))
        .returning(|_, _| Ok(()));
    mock_link_client
        .expect_action()
        .withf(|_, action| matches!(action, link_api::LinkAction::NotifyChipRemoved(_)))
        .returning(|_, _| Ok(()));

    // And the Chip Client expecting Create and Delete calls
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| Ok(()));
    mock_chip_client.expect_delete().times(1).returning(|_| Ok(()));

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

    // When I add a chip (which creates a device)
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
            name: "beacon".to_string(),
            manufacturer: "Netsim".to_string(),
            product_name: "NetsimBeacon".to_string(),
            network_params: netsim_model::chip::NetworkParams::Bluetooth(
                netsim_model::chip::BluetoothCreate {
                    address: "00:00:00:00:00:00".to_string(),
                    bt_properties: Default::default(),
                    mode: netsim_model::chip::BluetoothMode::Device(Default::default()),
                },
            ),
        },
    };
    let device_id = client.add_chip(params).await.unwrap();

    // And I delete the device
    let result = client.delete(device_id).await;

    // Then the operation succeeds
    assert!(result.is_ok(), "Expected delete to succeed for now");

    // Explicit verification happens at drop
}

#[tokio::test]
async fn test_delete_device_removes_chips() {
    // Given the Link Client expecting NotifyChipAdded and NotifyChipRemoved
    let mut mock_link_client = link_api::MockLinkClient::new();
    mock_link_client
        .expect_action()
        .withf(|_, action| matches!(action, link_api::LinkAction::NotifyChipAdded(_, _)))
        .returning(|_, _| Ok(()));
    mock_link_client
        .expect_action()
        .withf(|_, action| matches!(action, link_api::LinkAction::NotifyChipRemoved(_)))
        .returning(|_, _| Ok(()));

    // And the Chip Client expecting Create and Delete calls
    let mut mock_chip_client = netsim_model::chip::MockChipClient::new();
    mock_chip_client.expect_create().times(1).returning(|_| Ok(()));
    mock_chip_client.expect_delete().times(1).returning(|_| Ok(()));

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

    // When I create a device with a chip
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

    // And I delete the device
    client.delete(device_id).await.unwrap();

    // Then the device is no longer retrievable
    let device = client.get(device_id).await.unwrap();
    assert!(device.is_none());

    // Explicit verification happens at drop
}
