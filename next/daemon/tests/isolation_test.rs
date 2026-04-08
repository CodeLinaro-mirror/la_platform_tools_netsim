// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::world::World;

// Scenario: Global Reset Isolation
//   Given a running Netsim Daemon
//   And a built-in Default AP exists
//   And an internal CoffeeShop AP is created
//   And an internal mock device is created
//   When the global reset RPC is called
//   Then the internal CoffeeShop AP should be deleted
//   And the internal mock device should be deleted
//   And the built-in Default AP should still exist
#[tokio::test]
#[cfg(not(feature = "cuttlefish"))]
async fn test_global_reset_isolation() {
    let mut world = World::new().await;
    world.when_spawn_daemon().await;

    // Verify Default AP exists (Built-in)
    world.then_access_point_exists_by_ssid("AndroidWifi").await;

    // Create an internal CoffeeShop AP
    let coffeeshop_ap_id = world.when_create_access_point("CoffeeShop", 36, "a").await;

    // Create an internal mock device (No GUID)
    let mock_device_id = world.when_create_device("mock-device", "beacon").await;

    // Verify all exist before reset
    world.then_access_point_in_list(coffeeshop_ap_id).await;
    world.then_device_list_contains(mock_device_id, "mock-device").await;

    // When I call the reset RPC
    world.when_reset_is_called().await;

    // Then the internal CoffeeShop AP should be deleted
    world.then_access_point_not_in_list(coffeeshop_ap_id).await;

    // And the internal mock device should be deleted
    world.then_device_list_does_not_contain(mock_device_id).await;

    // And the built-in Default AP should still exist
    world.then_access_point_exists_by_ssid("AndroidWifi").await;
}
