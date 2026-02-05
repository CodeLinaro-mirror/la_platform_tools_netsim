// Copyright 2023-2025 The Android Open Source Project

use crate::world::World;

// Feature: Sniffer Support
//
//   As a client
//   I want to attach a sniffer
//   So that I can capture BLE advertisements

// Scenario: Sniffer receives advertisements
//
//   Given a beacon chip
//   And a sniffer chip attached
//   When the beacon advertises
//   Then the sniffer receives the advertisement packet
#[tokio::test]
async fn test_sniffer_receives_advertisement() {
    let mut world = World::new();

    // 1. Create a beacon.
    world.given_bluetooth_beacon("Beacon").await;

    // 2. Create a sniffer with the mock packet sink (managed by World).
    world.given_bluetooth_sniffer("Sniffer").await;

    // 3. Wait for the advertisement packet.
    let _packet = world.receive_packet("Sniffer").await;
}
