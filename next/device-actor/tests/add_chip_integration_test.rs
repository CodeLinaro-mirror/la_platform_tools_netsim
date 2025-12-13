#[path = "common/mod.rs"]
mod common;
// Copyright (C) 2025 The Android Open Source Project

// Integration tests for AddChip functionality.
//
// This module tests the AddChip API with a running DeviceActor and mocked ChipActor.
//
// List of tests:
// - `test_add_chip_success`: Verifies successful device creation via AddChip.
// - `test_add_chip_chip_failure`: Verifies proper error handling when chip creation fails during AddChip.

use actor_framework::ActorClient;
use device_api::DeviceAddChip;
use device_api::DeviceConfig;
use netsim_model::chip::ChipRequest;
use netsim_model::chip_error::ChipError;

use common::TestFixture;

fn create_add_chip_params(guid: &str, chip_name: &str) -> DeviceAddChip {
    DeviceAddChip {
        device_guid: guid.to_string(),
        packet_stream: None,
        packet_sink: None,
        device_config: DeviceConfig::new(
            "test-dev".to_string(),
            true,
            Default::default(),
            Default::default(),
        ),
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

#[tokio::test]
async fn test_add_chip_success() {
    let TestFixture { mut chip_rx, client, .. } = common::setup().await;

    // Mock chip service
    tokio::spawn(async move {
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
    });

    let params = create_add_chip_params("guid-1", "chip-1");
    let result = client.add_chip(params).await;
    assert!(result.is_ok(), "Add chip failed: {:?}", result.err());
    let device_id = result.unwrap();

    let device = client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.name, "test-dev");
}

#[tokio::test]
async fn test_add_chip_chip_failure() {
    let TestFixture { mut chip_rx, client, .. } = common::setup().await;

    // Mock chip service to fail
    tokio::spawn(async move {
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Err(ChipError::InvalidArguments("Mock failure".to_string()))).unwrap();
        }
    });

    let params = create_add_chip_params("guid-1", "chip-1");
    let result = client.add_chip(params).await;
    assert!(result.is_err());
}
