// Copyright (C) 2025 The Android Open Source Project

//! Mock tests for Device Client.
//!
//! This module uses a mock actor client to test DeviceClient logic without running a full actor.
//!
//! List of tests:
//! - `test_device_client_mock`: Verifies standard creation and action handling via mocks.
//! - `test_device_client_add_chip`: Verifies AddChip logic, including GUID mapping and chip addition.

use actor_framework::mock::MockClient;
use client::DeviceClient;
use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};

use device_api::DeviceActionResult;
use device_api::{DeviceConfig, DeviceId};
use netsim_model::chip::{BleBeacon, ChipId};

#[tokio::test]
async fn test_device_client_mock() {
    let mut mock = MockClient::<DeviceActor>::new();

    // Expect create
    let device_id = DeviceId(1);
    mock.expect_create().return_ok(device_id);

    // Expect action (Reset)
    mock.expect_action(device_id).return_ok(DeviceActionResult::Success);

    let client = DeviceClient::new(mock.client());

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

    mock.verify();
}

#[tokio::test]
async fn test_device_client_add_chip() {
    let mut mock = MockClient::<DeviceActor>::new();

    let device_id = DeviceId(1);
    let _chip_id1 = ChipId(1);
    let chip_id2 = ChipId(2);

    // 1. First add_chip (new device)
    // Expect create
    mock.expect_create().return_ok(device_id);

    let client = DeviceClient::new(mock.client());

    let params1 = create_add_chip_params("guid-1", "chip-1");
    client.add_chip(params1).await.unwrap();

    // 2. Second add_chip (same GUID, should add chip)
    mock.expect_action(device_id).return_ok(DeviceActionResult::ChipId(chip_id2));

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
