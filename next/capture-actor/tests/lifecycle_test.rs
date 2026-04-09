// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::ChipKind;

use crate::world::World;

// Feature: Capture Entity Lifecycle
//
//   As a client
//   I want to manage the lifecycle of a capture session
//   So that I can start, stop, and retrieve information about packet captures

// Scenario: Full lifecycle of a capture entity
#[tokio::test]
async fn test_capture_entity_lifecycle() {
    let world = World::new().await;
    let chip_id = 1;

    // Given: a capture is created but disabled
    world.when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device", false).await.unwrap();
    world.then_capture_is_enabled(chip_id, false).await;
    world.then_capture_stats_are(chip_id, 0, 0).await;

    // When: the capture is enabled
    world.when_update_capture(chip_id, true).await.unwrap();
    world.then_capture_is_enabled(chip_id, true).await;

    // And: a packet is sent
    world.when_dummy_packet_is_sent(chip_id).await;

    // Then: stats should be updated
    world.then_capture_stats_are(chip_id, 1, 4).await;

    // When: capture is disabled
    world.when_update_capture(chip_id, false).await.unwrap();
    world.then_capture_is_enabled(chip_id, false).await;

    // And: capture is deleted
    world.when_delete_capture(chip_id).await.unwrap();
    world.then_capture_is_none(chip_id).await;
}

// Scenario: Capture is enabled by default context setting
#[tokio::test]
async fn test_default_capture_enabled() {
    // Given: the actor is configured to enable captures by default
    let world = World::new_with_default(true).await;
    let chip_id = 2;

    // When: a capture is created with `enabled` = false
    world
        .when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device_default", false)
        .await
        .unwrap();

    // Then: it is actually enabled due to the context default
    world.then_capture_is_enabled(chip_id, true).await;
}

// Scenario: Capture file is created in the configured directory
#[tokio::test]
async fn test_capture_directory() {
    let world = World::new().await;
    let chip_id = 3;

    // When: an enabled capture is created
    world.when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device_dir", true).await.unwrap();

    // Then: a file is created in the temp directory
    world.then_capture_file_exists().await;
}
