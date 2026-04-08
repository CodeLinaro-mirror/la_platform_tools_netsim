// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use netsim_model::chip::ChipKind;

use crate::world::World;

// Feature: Capture Flushes
//
//   As a client
//   I want capture buffers to be flushed to disk
//   So that PCAP files contain recent packets even if the system crashes or
// stops

// Scenario: Capture flushes on periodic tick
#[tokio::test]
async fn test_capture_flushes_on_tick() {
    let world = World::new().await;
    let chip_id = 6;

    // Given: an enabled capture with a packet sent
    world
        .when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device_tick", true)
        .await
        .unwrap();
    world.when_dummy_packet_is_sent(chip_id).await;

    // File might be 0 bytes or have global header, but before flush,
    // the newly written bytes are buffered.
    // However, PcapWriter writes the global header immediately and might buffer the
    // rest. Wait for the tick interval (FLUSH_INTERVAL is 500ms)
    tokio::time::sleep(Duration::from_millis(600)).await;

    world.then_capture_file_is_not_empty().await;
}

// Scenario: Capture flushes on disable
#[tokio::test]
async fn test_capture_flushes_on_disable() {
    let world = World::new().await;
    let chip_id = 8;

    world
        .when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device_disable", true)
        .await
        .unwrap();
    world.when_dummy_packet_is_sent(chip_id).await;

    // Disable capture, which should flush and close writer
    world.when_update_capture(chip_id, false).await.unwrap();

    world.then_capture_file_is_not_empty().await;
}

// Scenario: Capture flushes on delete
#[tokio::test]
async fn test_capture_flushes_on_delete() {
    let world = World::new().await;
    let chip_id = 9;

    world
        .when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device_delete", true)
        .await
        .unwrap();
    world.when_dummy_packet_is_sent(chip_id).await;

    // Delete capture, which should flush and close writer
    world.when_delete_capture(chip_id).await.unwrap();

    world.then_capture_file_is_not_empty().await;
}

// Scenario: Capture flushes on shutdown
#[tokio::test]
async fn test_capture_flushes_on_shutdown() {
    let world = World::new().await;
    let chip_id = 10;

    world
        .when_create_capture(chip_id, ChipKind::BLUETOOTH, "test_device_shutdown", true)
        .await
        .unwrap();
    world.when_dummy_packet_is_sent(chip_id).await;

    // Shutdown actor, which should flush and close all writers
    world.when_shutdown().await.unwrap();

    world.then_capture_file_is_not_empty().await;
}
