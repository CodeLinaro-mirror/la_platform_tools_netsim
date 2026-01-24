// Copyright 2025 The Android Open Source Project

use crate::world::World;
use netsim_model::chip::ChipUpdate;
use netsim_model::device::Position;

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
    let id = world.given_bluetooth_device().await;

    // When the position is updated
    let new_pos = Position { x: 10.0, y: 20.0, z: 30.0 };
    let update = ChipUpdate { position: Some(new_pos.clone()), ..Default::default() };

    let updated_chip = world.client.0.update(id, update).await.expect("update failed");

    // Then the chip reflects the new position
    assert_eq!(updated_chip.position, new_pos);

    // And the actor state is updated
    let fetched_chip = world.client.0.get(id).await.unwrap().unwrap();
    assert_eq!(fetched_chip.position, new_pos);
}
