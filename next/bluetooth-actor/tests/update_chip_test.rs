// Copyright 2025 The Android Open Source Project

use netsim_model::device::Position;

use crate::world::World;

// Feature: Update Chip Properties
//
//   As a client
//   I want to update bluetooth chip properties
//   So that I can simulate device movement and other changes

// Scenario: Update chip position
//
//   Given a bluetooth chip
//   When the position is updated
//   Then the chip reflects the new position
#[tokio::test]
async fn test_update_position() {
    let mut world = World::new();

    // Given a bluetooth chip
    world.given_device("A").await;

    // When the position is updated
    let new_pos = Position { x: 10.0, y: 20.0, z: 30.0 };
    world.when_update_chip_position("A", new_pos.clone()).await;

    // Then the chip reflects the new position
    world.then_chip_position_is("A", new_pos).await;
}
