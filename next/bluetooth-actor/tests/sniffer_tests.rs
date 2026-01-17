// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils::mock_sink;
use crate::world::World;
use netsim_model::chip::{BeaconParams, BleBeacon, BluetoothMode, ChipCreate, SnifferParams};
use tokio::time::{timeout, Duration};

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
    let beacon_id = world.next_chip_id();
    let create_beacon_params = ChipCreate {
        id: beacon_id,
        packet_stream: None,
        packet_sink: None,
        config: World::create_chip_config(
            beacon_id,
            BluetoothMode::Beacon(Box::new(BeaconParams { ble_beacon: BleBeacon::default() })),
        ),
        device_id: world.device_id,
    };
    world.when_create_chip(create_beacon_params).await.ok();

    // 2. Create a sniffer with the mock packet sink.
    let (sink, mut sink_rx) = mock_sink();
    let sniffer_id = world.next_chip_id();
    let create_sniffer_params = ChipCreate {
        id: sniffer_id,
        packet_stream: None,
        packet_sink: Some(sink),
        config: World::create_chip_config(sniffer_id, BluetoothMode::Sniffer(SnifferParams {})),
        device_id: world.device_id,
    };
    world.when_create_chip(create_sniffer_params).await.ok();

    // 3. Wait for the advertisement packet.
    timeout(Duration::from_secs(1), async {
        sink_rx.recv().await.unwrap();
    })
    .await
    .expect("Did not receive advertisement within 1 second");
}
