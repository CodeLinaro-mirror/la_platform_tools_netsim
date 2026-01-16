#[path = "common/mod.rs"]
mod common;
// Copyright (C) 2025 The Android Open Source Project

// Tests for Server Shutdown behavior.
//
// This module tests the server's ability to shut down on idle or when all chips are removed.
// Note: These tests are expected to fail currently as the actor framework does not yet support auto-shutdown.
//
// List of tests:
// - `test_server_shutdown_on_idle`: Verifies server shuts down after idle timeout.
// - `test_server_shutdown_on_last_chip_delete`: Verifies server shuts down after last chip is deleted.

use actor_framework::ActorClient;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};
use device_api::DeviceConfig;
use netsim_model::chip::{BleBeacon, ChipRequest};
use std::time::Duration;

use common::TestFixture;

#[tokio::test]
#[ignore = "Auto-shutdown not yet implemented in actor framework"]
async fn test_server_shutdown_on_idle() {
    let TestFixture { mut chip_rx, client, .. } = common::setup().await;

    // Wait for a short period. In a real scenario, this would be the idle timeout.
    // Since we don't have an idle timeout implemented yet, we just wait a bit.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // If idle timeout was implemented, actor_task should be finished.
    // assert!(actor_task.is_finished(), "Server should have shut down on idle");

    // Subsequent calls should fail
    let result = client.list().await;
    assert!(result.is_err(), "Call should fail after server shutdown");
}

#[tokio::test]
#[ignore = "Auto-shutdown not yet implemented in actor framework"]
async fn test_server_shutdown_on_last_chip_delete() {
    let TestFixture { mut chip_rx, client, mut mock_link_controller, .. } = common::setup().await;

    // Expect NotifyChipAdded then NotifyChipRemoved
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());

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

    // Wait for potential shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    // assert!(actor_task.is_finished(), "Server should have shut down after last chip delete");
}
