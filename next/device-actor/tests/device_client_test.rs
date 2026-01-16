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
//   Scenario: Client correctly serializes Create and Reset requests
//     Given a mock Actor Client expecting Create and Reset calls
//     When I call create_device on the client
//     Then the mock receives the Create request
//     When I call reset on the client
//     Then the mock receives the PerformAction(Reset) request
#[tokio::test]
async fn test_device_client_mock() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();

    // Expect create
    let device_id = DeviceId(1);
    mock.expect_create().returning(move |_| Ok(device_id));

    // Expect action (Reset)
    mock.expect_perform_action()
        .withf(move |id, action| {
            *id == Some(device_id) && matches!(action, device_api::DeviceAction::Reset)
        })
        .returning(|_, _| Ok(DeviceActionResult::Success));

    let client = DeviceClient::new(Box::new(mock));

    // Test Create
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

    // Test Reset (Action)
    client.reset(id).await.unwrap();
}

// Scenario: client.add_chip manages device lifecycle
//   Given a mock Actor Client
//   When I call add_chip with a new device GUID
//   Then the client calls Create on the actor
//   When I call add_chip again with the SAME device GUID
//   Then the client calls PerformAction(AddChip) on the actor (reusing the device)
#[tokio::test]
async fn test_device_client_add_chip() {
    let mut mock = actor_framework::MockActorClient::<DeviceActor>::new();

    let device_id = DeviceId(1);
    let _chip_id1 = ChipId(1);
    let chip_id2 = ChipId(2);

    // 1. First add_chip (new device)
    // Expect create
    mock.expect_create().returning(move |_| Ok(device_id));

    // 2. Second add_chip (same GUID, should add chip)
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
