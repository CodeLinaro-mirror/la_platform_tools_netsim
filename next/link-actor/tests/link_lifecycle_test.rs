// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::chip::{ChipId, ChipKind};

use crate::world::World;

// Feature: Link Lifecycle
//
//   As a system
//   I want links to be managed consistent with chip lifecycle
//   So that no invalid links exist

// Scenario: Chip removal deletes associated links
//
//   Given a link exists
//   When one of the chips involved is removed
//   Then the link is automatically deleted
#[tokio::test]
async fn test_chip_removal_deletes_links() {
    // Given
    let world = World::new().await;
    world.given_default_chips().await;
    let link_id = world.when_create_link(ChipId(1), ChipId(2), -50).await.unwrap();
    assert_eq!(world.client.list().await.unwrap().len(), 1);

    // When
    world.when_notify_chip_removed(ChipId(1)).await;

    // Then
    let links = world.client.list().await.unwrap();
    assert!(links.is_empty(), "Link should be deleted after chip removal");

    // Check that we cannot update the deleted link
    let update_result = world.when_update_link(link_id, -60).await;
    assert!(update_result.is_err(), "Should not be able to update deleted link");
}

// Scenario: Cannot create link with removed chip
//
//   Given a chip was removed
//   When request to create a link involving that chip
//   Then the creation fails
#[tokio::test]
async fn test_create_link_fails_after_remove() {
    // Given
    let world = World::new().await;
    world.given_default_chips().await;
    // Ensure chip 1 exists (setup provides it)
    world.when_notify_chip_removed(ChipId(1)).await;

    // When
    let result = world.when_create_link(ChipId(1), ChipId(2), -60).await;

    // Then
    assert!(result.is_err());
}

// Scenario: Late chip addition enables link creation
//
//   Given a new chip is added dynamically
//   When request to create a link involving the new chip
//   Then the link is created successfully
#[tokio::test]
async fn test_chip_added_lifecycle() {
    // Given
    let world = World::new().await;
    world.given_default_chips().await;
    assert!(world.when_create_link(ChipId(99), ChipId(2), -50).await.is_err());

    // When
    world.when_notify_chip_added(ChipId(99), ChipKind::BLUETOOTH).await;

    // Then
    world.when_create_link(ChipId(99), ChipId(2), -50).await.unwrap();
}
