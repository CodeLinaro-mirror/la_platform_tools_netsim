// Copyright 2023-2025 The Android Open Source Project

use crate::world;

// Feature: Beacon Support
//
//   As a client
//   I want to create beacon chips
//   So that I can simulate BLE beacons

// Scenario: Successfully create a beacon chip
//
//   Given the bluetooth actor is running
//   When request to create a beacon chip
//   Then the chip is created successfully
#[tokio::test]
async fn test_add_chip() {
    let mut world = world::World::new();

    // Given a Bluetooth beacon
    world.given_bluetooth_beacon("Beacon").await;

    // Then the chip count should be 1
    world.then_chip_count_is(1).await;
}
