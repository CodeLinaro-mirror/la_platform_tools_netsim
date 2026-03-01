// Copyright (C) 2025 The Android Open Source Project

use crate::world::World;

// Scenario: Stats are written to file on shutdown
//   Given a running Device Actor with a stats file path
//   When I add multiple devices
//   And I shut down the actor (drop World)
//   Then the global stats are written to the file
//   And the stats contain the correct device count, peak count, and version
#[tokio::test]
async fn test_stats_persistence_on_shutdown() {
    let (path, _) = World::temp_stats_path();
    {
        let world = World::new_with_stats(path.clone(), None).await;
        world.when_add_chip("guid-1", "beacon").await;
        world.when_add_chip("guid-2", "beacon-2").await;
        world.when_shutdown_actor().await;
    }
    World::verify_stats_file_content(&path, "0.0.0-test", 2, 2).await;
}

// Scenario: Peak concurrent devices are tracked correctly
//   Given a running Device Actor
//   When I add 5 devices
//   And I remove 2 devices
//   And I shut down the actor
//   Then the stats file shows 3 active devices and 5 peak concurrent devices
#[tokio::test]
async fn test_peak_concurrent_persistence() {
    let (path, _) = World::temp_stats_path();
    {
        let world = World::new_with_stats(path.clone(), None).await;

        let mut ids = Vec::new();
        for i in 0..5 {
            let name = format!("device-{}", i);
            ids.push(world.when_create_device(&name).await);
        }

        world.when_delete_device(ids[3]).await;
        world.when_delete_device(ids[4]).await;
        world.when_shutdown_actor().await;
    }
    // Note: device_count in proto is cumulative (legacy behavior)
    World::verify_stats_file_content(&path, "0.0.0-test", 5, 5).await;
}

// Scenario: Stats are saved periodically (every 100ms)
//   Given a running Device Actor with a stats file path and 100ms interval
//   When I wait for the stats tick (300ms)
//   Then the global stats are written to the file
#[tokio::test]
async fn test_stats_periodic_save() {
    let (path, _) = World::temp_stats_path();
    let _world =
        World::new_with_stats(path.clone(), Some(std::time::Duration::from_millis(100))).await;

    // Wait for at least one tick (100ms) plus buffer
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    World::verify_stats_file_content(&path, "0.0.0-test", 0, 0).await;
}

// Scenario: A failed stats write cleans up the temporary file properly
//   Given a running Device Actor with a stats file path in a read-only
// directory   When the actor tries to save stats
//   Then the write should fail gracefully
//   And no orphaned .tmp file should remain in the directory
#[tokio::test]
async fn test_stats_write_failure_cleans_up_tmp_file() {
    let (path, _) = World::temp_stats_path();

    // Create a read-only directory to force a write error
    let mut bad_dir = path.clone();
    bad_dir.pop();
    bad_dir.push("readonly_stats_dir_test");
    std::fs::create_dir_all(&bad_dir).unwrap();

    // Make it read-only (Unix specific for this test)
    let mut perms = std::fs::metadata(&bad_dir).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&bad_dir, perms).unwrap();

    let bad_path = bad_dir.join("stats.json");
    let bad_tmp_path = bad_dir.join("stats.json.tmp");

    // The Stats::new should succeed but the subsequent tick should fail cleanly
    // without panicking and without leaving a .tmp file.
    let _world =
        World::new_with_stats(bad_path.clone(), Some(std::time::Duration::from_millis(50))).await;

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Verify .tmp file does NOT exist
    assert!(!bad_tmp_path.exists(), "Temporary file leaked on write failure!");

    // Clean up
    let mut perms = std::fs::metadata(&bad_dir).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&bad_dir, perms).unwrap();
    let _ = std::fs::remove_dir_all(&bad_dir);
}
