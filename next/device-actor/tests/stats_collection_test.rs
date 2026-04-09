// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// Scenario: Radio stats are collected from transport (DeviceActor)
//   Given a running Device Actor
//   And a device with a transport stream
//   When I send packets to the transport
//   Then the radio stats reflect the transport packets
use crate::world::World;

#[tokio::test]
async fn test_transport_stats() {
    let mut world = World::new().await;

    world.given_wifi_device_with_transport_stream("device-GUID", "chip-1").await;

    // Prime the mock stats (otherwise read_statistics returns empty result and
    // merge is skipped)
    world.given_mock_radio_stats(0, 0);

    // Send 3 packets of 10 bytes each -> 30 bytes total
    world.when_send_packets_to_transport(3, 10).await;

    world.when_fetch_radio_stats().await;

    // Transport Rx (Stream) -> Radio Tx (Air)
    // We sent 30 bytes into the stream (DeviceActor Rx), so it should appear as Tx
    // in Radio stats.
    world.then_radio_stats_should_match(30, 0);
}

#[tokio::test]
async fn test_stats_present_without_streams() {
    let mut world = World::new().await;

    // Create a device/chip WITHOUT transport stream (simulate UWB/WiFi internal
    // chips)
    // Use Wifi to avoid ambiguous Bluetooth fallback logic
    let device_id = world.when_add_wifi_chip("no-stream-guid", "chip-no-stream").await;

    // Ensure mock returns empty stats (simulate no internal stats)
    // By default `radio_stats` is empty, so `read_statistics` returns [].

    world.when_fetch_radio_stats().await;

    // Verify stats are present even if counts are 0
    let stats = world.last_radio_stats.as_ref().expect("No stats fetched");
    let device_stats = stats.iter().find(|s| s.id == device_id.0);

    assert!(device_stats.is_some(), "Stats should be present for chip without streams");
    let s = device_stats.unwrap();
    assert_eq!(s.tx_bytes, 0);
    assert_eq!(s.rx_bytes, 0);
}

#[tokio::test]
async fn test_stats_kind_mapping() {
    let mut world = World::new().await;

    // Add WiFi chip
    let wifi_id = world.when_add_wifi_chip("wifi-guid", "chip-wifi").await;
    // Add UWB chip
    let uwb_id = world.when_add_uwb_chip("uwb-guid", "chip-uwb").await;

    // Fetch stats
    world.when_fetch_radio_stats().await;

    let stats = world.last_radio_stats.as_ref().expect("No stats fetched");

    // Verify WiFi
    let wifi_stats = stats.iter().find(|s| s.id == wifi_id.0).expect("WiFi stats missing");
    assert_eq!(wifi_stats.kind, netsim_model::RadioKind::Wifi, "WiFi kind should be Wifi");

    // Verify UWB
    let uwb_stats = stats.iter().find(|s| s.id == uwb_id.0).expect("UWB stats missing");
    assert_eq!(uwb_stats.kind, netsim_model::RadioKind::Uwb, "UWB kind should be Uwb");
}
