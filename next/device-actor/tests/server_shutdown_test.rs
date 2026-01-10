// Copyright (C) 2025 The Android Open Source Project

// Feature: Server Shutdown
//
//   As a system administrator or developer
//   I want the device actor server to shut down when idle or when requested
//   So that resources are released when not in use
//
//   Scenario: Server shuts down after idle timeout
//     Given a running Device Actor
//     When the server is idle for a duration
//     Then the server shuts down automatically
//
//   Scenario: Server shuts down when last chip is deleted
//     Given a running Device Actor with one chip
//     When I delete the last chip
//     Then the server shuts down automatically

use device_actor::DeviceActor;
use device_api::api::{Chip, DeviceChipCreate, DeviceCreate};
use device_api::DeviceConfig;
use netsim_model::chip::BleBeacon;
use netsim_model::chip::{ChipClient, MockChipClient, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
#[ignore = "Auto-shutdown not yet implemented in actor framework"]
async fn test_server_shutdown_on_idle() {
    // Given a running Device Actor
    let (mut mock_link_controller, mock_link_client) = link_api::mock::MockLinkClient::new();
    // Expect generic action if needed, or none
    // mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());

    // Inline setup
    let mut chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>> = HashMap::new();
    let chip_client = MockChipClient::new();
    chip_clients.insert(NetworkKind::Bluetooth, Box::new(chip_client));

    let (runner, client) = device_actor::new();
    let actor = DeviceActor::new(
        chip_clients,
        Arc::new(AtomicU32::new(0)),
        None,
        Box::new(mock_link_client),
    );
    tokio::spawn(runner.run(actor));

    // When the server is idle for a duration
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Then the server shuts down automatically
    // If idle timeout was implemented, actor_task should be finished.
    // assert!(actor_task.is_finished(), "Server should have shut down on idle");

    // Subsequent calls should fail
    let result = client.list().await;
    assert!(result.is_err(), "Call should fail after server shutdown");
}

#[tokio::test]
#[ignore = "Auto-shutdown not yet implemented in actor framework"]
async fn test_server_shutdown_on_last_chip_delete() {
    // Given a running Device Actor with one chip
    let (mut mock_link_controller, mock_link_client) = link_api::mock::MockLinkClient::new();
    // Expect NotifyChipAdded
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());
    // Expect NotifyChipRemoved
    mock_link_controller.expect_action(link_api::LinkId(0)).return_ok(());

    // Inline setup
    let mut chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>> = HashMap::new();
    let mut chip_client = MockChipClient::new();
    chip_client.expect_create().returning(|_| Ok(()));
    chip_client.expect_delete().returning(|_| Ok(()));

    chip_clients.insert(NetworkKind::Bluetooth, Box::new(chip_client));

    let (runner, client) = device_actor::new();
    let actor = DeviceActor::new(
        chip_clients,
        Arc::new(AtomicU32::new(0)),
        None,
        Box::new(mock_link_client),
    );
    tokio::spawn(runner.run(actor));

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

    // When I delete the last chip
    client.delete(device_id).await.unwrap();

    // Then the server shuts down automatically
    // Wait for potential shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    // assert!(actor_task.is_finished(), "Server should have shut down after last chip delete");
}
