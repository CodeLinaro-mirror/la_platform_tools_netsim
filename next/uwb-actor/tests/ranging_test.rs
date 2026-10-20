// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::Position;
use pica::packets::uci;

use crate::world::World;

// Feature: UWB Ranging between chips
//
//   As a client
//   I want to place two UWB chips at different positions
//   And establish a ranging session between them
//   So that I can verify that pica computes the correct range

// Scenario: Ranging between two chips at 10 meters distance
//
//   Given a chip 100 at (0, 0, 0)
//   And a chip 200 at (10, 0, 0)
//   When UCI session 1 is established on chip 100 as CONTROLLER with peer mac
// [0, 1]   And UCI session 1 is established on chip 200 as CONTROLEE with peer
// mac [0, 0]   And ranging is started on chip 100 for session 1
//   Then a ranging measurement of 1000 cm is received on chip 100
#[tokio::test]
async fn test_ranging_between_two_chips() {
    let mut world = World::new().await;

    let chip_a = 1;
    let chip_a_mac = [0, 0];
    let chip_b = 2;
    let chip_b_mac = [0, 1];

    // Given
    world.given_a_chip_at(chip_a, Position { x: 0.0, y: 0.0, z: 0.0 }).await;
    world.given_a_chip_at(chip_b, Position { x: 10.0, y: 0.0, z: 0.0 }).await;

    // Clear initial READY notifications and RESET
    world.then_status_notification_is_received(chip_a, uci::DeviceState::DeviceStateReady).await;
    world.when_uci_is_reset(chip_a).await;

    world.then_status_notification_is_received(chip_b, uci::DeviceState::DeviceStateReady).await;
    world.when_uci_is_reset(chip_b).await;

    // When
    world
        .when_uci_session_is_established(
            chip_a,
            1,
            uci::DeviceType::Controller,
            uci::DeviceRole::Initiator,
            chip_a_mac,
            chip_b_mac,
        )
        .await;

    world
        .when_uci_session_is_established(
            chip_b,
            1,
            uci::DeviceType::Controlee,
            uci::DeviceRole::Responder,
            chip_b_mac,
            chip_a_mac,
        )
        .await;

    world.when_ranging_is_started(chip_a, 1).await;
    world.when_ranging_is_started(chip_b, 1).await;

    // Trigger one ranging round manually
    world.when_ranging_is_triggered(chip_a, 1).await;

    // Then
    // distance = 1m = 1000cm
    world.then_ranging_measurement_is_received(chip_a, 1000).await;

    // Verify telemetry
    use netsim_model::ChipClient;
    let stats = world.client.read_statistics().await.expect("Failed to read statistics");
    let stats_a = stats.iter().find(|s| s.id == chip_a).unwrap();
    let stats_b = stats.iter().find(|s| s.id == chip_b).unwrap();
    assert_eq!(stats_a.p2p_tx_count, 1);
    assert_eq!(stats_a.p2p_rx_count, 1);
    assert_eq!(stats_b.p2p_tx_count, 0);
    assert_eq!(stats_b.p2p_rx_count, 0);
}
