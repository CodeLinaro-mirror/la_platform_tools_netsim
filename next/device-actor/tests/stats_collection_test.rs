// Scenario: Radio stats are collected from transport (DeviceActor)
//   Given a running Device Actor
//   And a device with a transport stream
//   When I send packets to the transport
//   Then the radio stats reflect the transport packets
use crate::world::World;

#[tokio::test]
async fn test_transport_stats() {
    let mut world = World::new().await;

    world.given_device_with_transport_stream("device-GUID", "chip-1").await;

    // Prime the mock stats (otherwise read_statistics returns empty result and
    // merge is skipped)
    world.given_mock_radio_stats(0, 0);

    // Send 3 packets of 10 bytes each -> 30 bytes total
    world.when_send_packets_to_transport(3, 10).await;

    world.when_fetch_radio_stats().await;

    // Transport Rx (Stream) -> Radio Tx (Air)
    // We sent 30 bytes into the stream (DeviceActor Rx), so it should appear as Tx
    // in Radio stats.
    world.then_radio_stats_should_match(30, 0);
}
