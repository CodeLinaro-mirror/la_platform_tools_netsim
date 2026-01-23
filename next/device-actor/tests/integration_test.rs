#[path = "common/mod.rs"]
mod common;
// Copyright (C) 2025 The Android Open Source Project

// Integration tests for Device Actor.
//
// This module tests the integration between DeviceClient, DeviceActor, and ChipActor (mocked).
// It covers standard device lifecycle operations.
//
// List of tests:
// - `test_create_device_succeeds`: Verifies standard device creation and chip propagation.
// - `test_delete_device_removes_chips`: Verifies that deleting a device also deletes its chips.
// - `test_update_device_propagates_to_chips`: Verifies that updating a device propagates changes to chips.
// - `test_notify_chip_removed`: Verifies that removing a chip updates the device state.

use actor_framework::ActorClient;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate, DeviceUpdate};
use device_api::DeviceConfig;
use netsim_model::chip::{BleBeacon, ChipId, ChipRequest};

use common::{setup, TestFixture};

#[tokio::test]
async fn test_create_device_succeeds() {
    let TestFixture { mut chip_rx, client, .. } = common::setup().await;

    // Mock chip service
    tokio::spawn(async move {
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
    });

    let device_name = "test-dev-1".to_string();
    let chip_config = DeviceChipCreate {
        name: "beacon".to_string(),
        manufacturer: "Netsim".to_string(),
        product_name: "NetsimBeacon".to_string(),
        chip: Chip::Beacon(BleBeacon::default()),
    };
    let params = DeviceCreate {
        device_config: DeviceConfig::new(
            device_name.clone(),
            true,
            Default::default(),
            Default::default(),
        ),
        chip: chip_config,
    };

    let device_id = client.create_device(params).await.unwrap();
    let device = client.get(device_id).await.unwrap().unwrap();

    assert_eq!(device.name, device_name);
    assert_eq!(device.chips.len(), 1);
}

#[tokio::test]
async fn test_delete_device_removes_chips() {
    let TestFixture { mut chip_rx, client, .. } = setup().await;

    // Mock chip service for create and delete
    tokio::spawn(async move {
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
        if let Some(ChipRequest::Delete { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
    });

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
    client.delete(device_id).await.unwrap();

    let device = client.get(device_id).await.unwrap();
    assert!(device.is_none());
}

#[tokio::test]
async fn test_update_device_propagates_to_chips() {
    let TestFixture { mut chip_rx, client, .. } = setup().await;

    // Mock chip service for create and update
    tokio::spawn(async move {
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
        if let Some(ChipRequest::Update { patch, respond_to, .. }) = chip_rx.recv().await {
            assert!(patch.position.is_some());
            assert!(patch.orientation.is_some());
            respond_to.send(Ok(netsim_model::chip::Chip::default())).unwrap();
        }
    });

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
    update.position = Some(device_api::Position { x: 10.0, y: 10.0, z: 0.0 });
    update.orientation = Some(device_api::Orientation { yaw: 1.0, pitch: 0.0, roll: 0.0 });
    update.name = Some("updated-name".to_string());

    client.update(device_id, update).await.unwrap();

    let device = client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.position.x, 10.0);
    assert_eq!(device.orientation.yaw, 1.0);
    assert_eq!(device.name, "updated-name");
}

#[tokio::test]
async fn test_notify_chip_removed() {
    let TestFixture { mut chip_rx, client, .. } = setup().await;

    // Mock chip service for create
    tokio::spawn(async move {
        if let Some(ChipRequest::Create { respond_to, .. }) = chip_rx.recv().await {
            respond_to.send(Ok(())).unwrap();
        }
    });

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

    client.notify_chip_removed(device_id, chip_id).await.unwrap();

    let device = client.get(device_id).await.unwrap().unwrap();
    assert_eq!(device.chips.len(), 0);
}
