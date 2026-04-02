// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

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
    world.given_beacon("Beacon").await;

    // Then the chip count should be 1
    world.then_chip_count_is(1).await;
}

// Scenario: Beacon created without a name gets Beacon-{ID}
#[tokio::test]
async fn test_beacon_unique_default_naming() {
    let mut world = world::World::new();

    // When a beacon is created with defaults (empty name)
    let id = world.when_create_beacon_with_defaults().await;

    // Then it should have the name Beacon-{ID}
    world.then_chip_name_is(id, &format!("Beacon-{}", id.0)).await;
}

// Scenario: Multiple beacons created with defaults get unique names
#[tokio::test]
async fn test_multiple_beacons_unique_naming() {
    let mut world = world::World::new();

    // When two beacons are created with defaults
    let id1 = world.when_create_beacon_with_defaults().await;
    let id2 = world.when_create_beacon_with_defaults().await;

    // Then they should have unique names
    world.then_chip_name_is(id1, &format!("Beacon-{}", id1.0)).await;
    world.then_chip_name_is(id2, &format!("Beacon-{}", id2.0)).await;
}

// Scenario: Beacon created with explicit name preserves it
#[tokio::test]
async fn test_beacon_explicit_name() {
    let mut world = world::World::new();

    // When a beacon is created with an explicit name "MyBeacon"
    world.given_beacon("MyBeacon").await;

    // Then it should keep that name
    let id = *world.chips.get("MyBeacon").unwrap();
    world.then_chip_name_is(id, "MyBeacon").await;
}
