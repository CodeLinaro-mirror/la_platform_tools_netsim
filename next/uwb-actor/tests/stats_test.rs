#[allow(unused_imports)]
use netsim_model::stats::NetsimRadioStats;
use netsim_model::{chip::ChipClient, ChipKind};

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
    let expected_stats = [NetsimRadioStats {
        name: format!("uwb_chip_{}", chip_id),
        id: chip_id,
        kind: netsim_model::stats::RadioKind::Uwb,
        tx_bytes: 0,
        rx_bytes: 0,
        ..Default::default()
    }];
    assert_eq!(&*stats, &expected_stats[..]);
}
