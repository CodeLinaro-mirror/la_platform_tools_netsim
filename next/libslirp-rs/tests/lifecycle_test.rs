// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::world::World;

// Feature: LibSlirp Lifecycle
//
//   As a developer
//   I want to initialize and shutdown the LibSlirp network stack
//   So that resources and worker threads are cleanly released

// Scenario: Clean stack shutdown disconnects packet channel
//   Given a running LibSlirp stack
//   When the network stack is shut down
//   Then the packet receiver channel is disconnected
#[test]
fn test_shutdown() {
    // Given
    let mut world = World::new();
    world.given_default_slirp();

    // When
    world.when_shutdown_slirp();

    // Then
    world.then_receiver_disconnected();
}
