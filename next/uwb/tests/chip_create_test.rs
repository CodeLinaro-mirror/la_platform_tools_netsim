// Copyright 2026 The Android Open Source Project

use crate::world::World;
use netsim_model::chip::{ChipId, ChipKind};
use netsim_model::chip_error::ChipError;

// Feature: UWB Chip Creation
//
//   As a client
//   I want to create UWB chips
//   So that I can simulate UWB devices

// Scenario: Successfully create and retrieve a UWB chip
//
//   Given the UWB actor is running
//   When request to create a chip
//   Then the chip is created and can be retrieved with correct properties
#[tokio::test]
async fn test_create_and_get_chip() {
    // Given
    let world = World::new().await;
    let chip_id = 1;

    // When
    world.when_create_chip(chip_id).await.unwrap();

    // Then
    let chip_info = world.when_get_chip(chip_id).await.unwrap();
    match chip_info.variant {
        Some(netsim_model::chip::ChipVariant::Uwb(_)) => {
            assert_eq!(chip_info.id, chip_id);
            assert_eq!(chip_info.kind, ChipKind::UWB);
            assert_eq!(chip_info.name, Some(format!("uwb_chip_{}", chip_id)));
        }
        _ => panic!("Unexpected ChipInfo variant"),
    }
}

// Scenario: Fail to create duplicate chip
//
//   Given a chip already exists
//   When request to create another chip with the same ID
//   Then the creation fails with ChipExists error
#[tokio::test]
async fn test_create_duplicate_chip() {
    // Given
    let world = World::new().await;
    let chip_id = 2;
    world.when_create_chip(chip_id).await.unwrap();

    // When
    let result = world.when_create_chip(chip_id).await;

    // Then
    match result {
        Err(ChipError::ChipExists(id)) => {
            assert_eq!(id, ChipId(chip_id).0);
        }
        _ => panic!("Expected ChipExists error, got {:?}", result),
    }
}
