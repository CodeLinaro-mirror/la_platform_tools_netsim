// Copyright 2023-2025 The Android Open Source Project

use crate::world::World;

// Feature: Scanner Support
//
//   As a client
//   I want to attach a scanner
//   So that I can capture BLE advertisements

// Scenario: Scanner receives advertisements
//
//   Given a beacon chip
//   And a scanner chip attached
//   When the beacon advertises
//   Then the scanner receives the advertisement packet
#[tokio::test]
async fn test_scanner_receives_advertisement() {
    let mut world = World::new();

    // 1. Create a beacon on a DIFFERENT device to allow loopback/air traffic.
    world.given_beacon("Beacon").await;

    // 2. Create a scanner with the mock packet sink (managed by World).
    world.given_scanner("Scanner").await;

    // 3. Wait for the advertisement packet.
    world.then_scanner_sees_any_adv("Scanner").await;
}

// Scenario: Scanner receives Tx Power and RSSI
//
//   Given a beacon chip with Tx Power enabled
//   And a scanner chip attached
//   When the beacon advertises
//   Then the scanner receives the advertisement packet with RSSI and Tx Power
#[tokio::test]
async fn test_scanner_receives_tx_power_and_rssi() {
    let mut world = World::new();

    // 1. Create a beacon with Tx Power enabled
    world.given_beacon_with_tx_power("Beacon", "High").await;

    // 2. Create a scanner
    world.given_scanner("Scanner").await;

    // 3. Verify fields
    world.then_scanner_sees_adv_with_rssi("Scanner", "High").await;
}

// Scenario: Sink closure triggers device removal
//
//   Given a scanner chip
//   And a beacon chip (to generate traffic)
//   When the packet sink is dropped/closed
//   Then the scanner chip is removed
#[tokio::test]
async fn test_sink_closure_removes_chip() {
    let mut world = World::new();

    // 1. Create a scanner (managed by World).
    world.given_scanner("Scanner").await;

    // 2. Create a beacon (to generate traffic).
    world.given_beacon("Beacon").await;

    // 3. Drop the sink (simulate disconnection).
    world.when_sink_dropped("Scanner");

    // 4. Wait for chip removal.
    world.then_chip_eventually_removed("Scanner").await;
}

// Scenario: Scanner captures duplicates (sniffer mode)
//
//   Given a beacon chip
//   And a scanner chip attached
//   When the beacon continues to advertise
//   Then the scanner receives multiple reports for the same beacon
#[tokio::test]
async fn test_scanner_captures_duplicates() {
    let mut world = World::new();

    // 1. Create a beacon.
    world.given_beacon("Beacon").await;

    // 2. Create a scanner.
    world.given_scanner("Scanner").await;

    // 3. Verify we receive at least 3 reports.
    for _ in 0..3 {
        world.receive_scan_report("Scanner").await;
    }
}

// Scenario: Scanner sees multiple beacons
//
//   Given two beacon chips
//   And a scanner chip attached
//   When both beacons advertise
//   Then the scanner receives reports from both
#[tokio::test]
async fn test_scanner_sees_multiple_beacons() {
    let mut world = World::new();

    // 1. Create two beacons with different names/addresses managed by World (auto-assigned).
    world.given_beacon("Beacon1").await;
    world.given_beacon("Beacon2").await;

    // 2. Create a scanner.
    world.given_scanner("Scanner").await;

    // 3. Verify scanner sees both beacons.
    world.then_scanner_sees_adv_from("Scanner", "Beacon1").await;
    world.then_scanner_sees_adv_from("Scanner", "Beacon2").await;
}
