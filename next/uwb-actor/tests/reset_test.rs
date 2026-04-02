// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use pica::packets::uci;

use crate::world::World;

// Feature: UWB Chip Reset via Actor Client
//
//   As a client
//   I want to reset a UWB chip through the UwbClient
//   And receive the expected UCI responses from Pica
//   So that I can verify the end-to-end reset flow

// Scenario: Reset UWB chip and receive response/notification
//
//   Given the UWB actor is running with Pica
//   And a UWB chip is created
//   When the chip is reset
//   Then a CORE_DEVICE_RESET_RSP(OK) is received
//   And a CORE_DEVICE_STATUS_NTF(READY) is received
#[tokio::test]
async fn test_chip_reset() {
    // Given
    let mut world = World::new().await;
    let chip_id = 1;
    world.given_a_chip(chip_id).await;

    // When
    world.when_chip_is_reset(chip_id).await;

    // Then
    world.then_reset_response_is_received(chip_id, uci::Status::Ok).await;
    world.then_status_notification_is_received(chip_id, uci::DeviceState::DeviceStateReady).await;
}
