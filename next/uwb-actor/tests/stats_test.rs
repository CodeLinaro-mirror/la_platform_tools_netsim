// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::{chip::ChipClient, stats::NetsimRadioStats};

use crate::world::World;

#[tokio::test]
async fn test_read_statistics() {
    let mut world = World::new().await;

    // Given: A UWB chip is created
    let chip_id = 1;
    world.given_a_chip(chip_id).await;

    // When: client reads statistics
    let stats = world.client.read_statistics().await.expect("Failed to read statistics");

    // Then: stats contain the chip with zero counts
    let mut expected_stat = NetsimRadioStats::default();
    expected_stat.name = format!("uwb_chip_{}", chip_id);
    expected_stat.id = chip_id;
    expected_stat.kind = netsim_model::stats::RadioKind::Uwb;
    let expected_stats = [expected_stat];
    assert_eq!(&*stats, &expected_stats[..]);
}
