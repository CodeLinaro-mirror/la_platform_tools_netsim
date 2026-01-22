// Copyright (C) 2025 The Android Open Source Project

//! Device Client Logic Tests.
//!
//! This module uses a mock actor client to verify the `DeviceClient` wrapper logic,
//! ensuring correct serialization and state management without a full actor runtime.

use client::DeviceClient;
use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};

use device_api::DeviceActionResult;
use device_api::{DeviceConfig, DeviceId};
use netsim_model::chip::{BleBeacon, ChipId};

// Feature: Device Client Logic
//
//   As a developer
//   I want to verify the implementation of the Device Client
//   So that I can ensure it correctly communicates with the actor
//
// Scenario: Client correctly serializes Create requests
//   Given a mock Actor Client expecting a Create call
//   When I call create_device on the client
//   Then the mock receives the Create request
#[tokio::test]
async fn test_device_client_create() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();
    let device_id = DeviceId(1);

    mock.expect_create().returning(move |_| Ok(device_id));

    let client = DeviceClient::new(Box::new(mock));

    let params = DeviceCreate {
        device_config: DeviceConfig::new(
            "test".to_string(),
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
    let id = client.create_device(params).await.unwrap();
    assert_eq!(id, device_id);
}

// Scenario: Client correctly serializes Reset requests
//   Given a mock Actor Client expecting a PerformAction(Reset) call with a specific ID
//   When I call reset on the client with a device ID
//   Then the mock receives the PerformAction(Reset) request with that ID
#[tokio::test]
async fn test_device_client_reset() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();
    let device_id = DeviceId(1);

    mock.expect_perform_action()
        .withf(move |id, action| {
            *id == Some(device_id) && matches!(action, device_api::DeviceAction::Reset)
        })
        .returning(|_, _| Ok(DeviceActionResult::Success));

    let client = DeviceClient::new(Box::new(mock));
    client.reset(Some(device_id)).await.unwrap();
}

// Scenario: Client correctly serializes Global Reset requests
//   Given a mock Actor Client expecting a PerformAction(Reset) call with None ID
//   When I call reset on the client with None
//   Then the mock receives the PerformAction(Reset) request with None
#[tokio::test]
async fn test_device_client_global_reset() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();

    mock.expect_perform_action()
        .withf(move |id, action| id.is_none() && matches!(action, device_api::DeviceAction::Reset))
        .returning(|_, _| Ok(DeviceActionResult::Success));

    let client = DeviceClient::new(Box::new(mock));
    client.reset(None).await.unwrap();
}

// Scenario: client.add_chip creates a new device when GUID is unknown
//   Given a mock Actor Client expecting a Create call
//   When I call add_chip with a new device GUID
//   Then the client calls Create on the actor
#[tokio::test]
async fn test_device_client_add_chip_new_device() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();
    let device_id = DeviceId(1);

    mock.expect_create().returning(move |_| Ok(device_id));

    let client = DeviceClient::new(Box::new(mock));
    let params = create_add_chip_params("guid-1", "chip-1");

    client.add_chip(params).await.unwrap();
}

// Scenario: client.add_chip adds a chip to an existing device when GUID is known
//   Given a mock Actor Client that has already created a device for GUID X
//   When I call add_chip again with the SAME device GUID X
//   Then the client calls PerformAction(AddChip) on the actor
#[tokio::test]
async fn test_device_client_add_chip_existing_device() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();
    let device_id = DeviceId(1);
    let chip_id2 = ChipId(2);

    mock.expect_create().returning(move |_| Ok(device_id));

    mock.expect_perform_action()
        .withf(move |id, action| {
            *id == Some(device_id) && matches!(action, device_api::DeviceAction::AddChip { .. })
        })
        .returning(move |_, _| Ok(DeviceActionResult::ChipId(chip_id2)));

    let client = DeviceClient::new(Box::new(mock));

    let params1 = create_add_chip_params("guid-1", "chip-1");
    client.add_chip(params1).await.unwrap();

    let params2 = create_add_chip_params("guid-1", "chip-2");
    client.add_chip(params2).await.unwrap();
}

fn create_add_chip_params(guid: &str, chip_name: &str) -> device_api::DeviceAddChip {
    device_api::DeviceAddChip {
        device_guid: guid.to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig::default(),
        chip_config: netsim_model::chip::ChipConfig {
            name: chip_name.to_string(),
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
    }
}
