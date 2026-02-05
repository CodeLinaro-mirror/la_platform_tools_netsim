// Copyright 2026 The Android Open Source Project

use crate::world::World;
use netsim_model::chip::ChipId;
use netsim_model::chip_error::ChipError;

// Feature: UWB Chip Lifecycle
//
//   As a client
//   I want to manage the lifecycle of UWB chips
//   So that I can simulate chips appearing and disappearing

// Scenario: Retrieve non-existent chip fails
//
//   Given the UWB actor is running
//   When request to get a chip that does not exist
//   Then the retrieval fails with ChipNotFound error
#[tokio::test]
async fn test_get_chip_not_found() {
    // Given
    let mut world = World::new().await;
    let chip_id = 99;

    // When
    let result = world.when_get_chip(chip_id).await;

    // Then
    match result {
        Err(ChipError::ChipNotFound(id)) => {
            assert_eq!(id, ChipId(chip_id));
        }
        _ => panic!("Expected ChipNotFound error, got {:?}", result),
    }
}

// Scenario: Delete chip successfully
//
//   Given a chip exists
//   When request to delete the chip
//   Then the chip is deleted and cannot be retrieved
#[tokio::test]
async fn test_delete_chip() {
    // Given
    let mut world = World::new().await;
    let chip_id = 3;
    world.given_a_chip(chip_id).await;

    // When
    world.when_delete_chip(chip_id).await.unwrap();

    // Then
    world.then_chip_does_not_exist(chip_id).await;
}

// Scenario: Chip is deleted when its packet stream is closed
//
//   Given a chip exists
//   When its packet stream is closed
//   Then the chip is automatically deleted
#[tokio::test]
async fn test_chip_deleted_on_stream_close() {
    // Given
    let mut world = World::new().await;
    let chip_id = 4;
    world.given_a_chip(chip_id).await;
    world.then_chip_exists(chip_id).await;

    // When
    world.and_packet_stream_is_closed(chip_id);

    // Then
    world.then_chip_does_not_exist(chip_id).await;
}
