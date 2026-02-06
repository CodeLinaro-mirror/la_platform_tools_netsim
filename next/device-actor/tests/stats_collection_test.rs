// Copyright (C) 2025 The Android Open Source Project

use crate::world::World;

// Scenario: Radio stats are collected from chips
//   Given a running Device Actor
//   And a device with radio stats
//   When I poll for radio stats
//   Then the radio stats contain the device with correct bytes
#[tokio::test]
async fn test_radio_stats() {
    let mut world = World::new().await;

    // Create a device first so it has an ID
    let device_id = world.when_create_device("device-1").await;
    let id_u32 = device_id.0;

    // Given a device with radio stats
    world.given_radio_stats(id_u32, 1000, 2000);

    // When I poll for radio stats
    world.when_fetch_radio_stats().await;

    // Then the radio stats contain the device with correct bytes
    world.then_radio_stats_has_device_with_bytes(id_u32, 1000, 2000);
}
