// Copyright 2025 The Android Open Source Project

use std::{collections::HashSet, time::Duration};

use crate::world::World;

#[tokio::test]
async fn beacon_scan_stress_test() {
    let mut world = World::new();
    let scanner_name = "Scanner";
    world.given_scanner(scanner_name).await;

    let num_beacons = 20;

    /*
    // Create a client to interact with the actor for noise production
    let client = world.client.clone();
    tokio::spawn(async move {
        for _ in 0..10000 {
            // Spam list commands to keep the actor loop busy (control-plane stress)
            let _ = client.0.list().await;
            tokio::task::yield_now().await;
        }
    });
    */

    // Add beacons to the world
    for i in 0..num_beacons {
        world.given_beacon(&format!("Beacon-{}", i)).await;
        // Small delay to prevent overwhelming the initial setup
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let mut seen_beacons = HashSet::new();
    let timeout = Duration::from_secs(20);
    let start = std::time::Instant::now();

    // Collect advertisements until all beacons are seen or timeout occurs
    while seen_beacons.len() < num_beacons && start.elapsed() < timeout {
        let reports = world.receive_scan_report(scanner_name).await;
        for report in reports {
            // Beacons created by World have address 00:00:00:00:HIGH:LOW.
            let id = ((report.mac[4] as u32) << 8) | (report.mac[5] as u32);
            seen_beacons.insert(id);
        }
    }

    let final_count = seen_beacons.len();
    println!("Seen beacons count: {}", final_count);

    // Verify all beacons were seen
    assert_eq!(
        final_count, num_beacons,
        "Lost BLE scan results! Seen {}/{}",
        final_count, num_beacons
    );
}
