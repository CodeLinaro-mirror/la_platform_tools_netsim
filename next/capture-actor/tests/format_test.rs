// Copyright 2026 The Android Open Source Project

use netsim_model::chip::ChipKind;

use crate::world::World;

// Feature: Capture Packet Formats
//
//   As a client
//   I want to capture packets for different chip types
//   So that PCAP files are generated with the correct format and encapsulation

// Scenario: UWB packet is captured correctly
#[tokio::test]
async fn test_uwb_capture() {
    let world = World::new().await;
    let chip_id = 4;

    // Given: an enabled UWB capture
    world.when_create_capture(chip_id, ChipKind::UWB, "test_uwb_device", true).await.unwrap();

    // When: a dummy packet is sent
    world.when_dummy_packet_is_sent(chip_id).await;

    // Then: the capture info reflects the written packet
    world.then_capture_stats_are(chip_id, 1, 4).await;
}

// Scenario: Wi-Fi packet handles Hwsim formatting
#[tokio::test]
async fn test_wifi_capture() {
    let world = World::new().await;
    let chip_id = 5;

    // Test both WIFI and AP to ensure service routes them correctly
    for kind in [ChipKind::WIFI, ChipKind::AP] {
        // Given: an enabled Wi-Fi or AP capture
        world
            .when_create_capture(chip_id, kind, &format!("test_{:?}_device", kind), true)
            .await
            .unwrap();

        // When: an invalid dummy payload is sent
        world.when_dummy_packet_is_sent(chip_id).await;

        // Then: it is discarded during parsing, resulting in 0 records
        world.then_capture_stats_are(chip_id, 0, 0).await;

        // Cleanup for next iteration
        world.when_delete_capture(chip_id).await.unwrap();
    }
}
