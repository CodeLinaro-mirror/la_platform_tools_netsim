use crate::world::World;

// ============================================================================
// Feature: Wifi Stats Counters and Errors
// ============================================================================

// Scenario: A Station (Chip) sends a malformed packet
// Given a Wifi Medium with a provisioned chip
// When the Station transmits a malformed packet
// Then the client_error stat is incremented
#[tokio::test]
async fn test_client_error_stat() {
    let mut world = World::new().await;
    let _sender = world.given_a_chip(1).await;

    world.when_global_stats_are_captured().await;
    world.when_chip_transmits_malformed_packet(0).await;
    world.then_client_errors_increased_by(1).await;
}

// Scenario: The infrastructural gateway rejects a packet
// Given a Wifi Medium with the real SlirpGateway
// When a chip transmits a payload that is not a valid Ethernet frame
// Then the frame_error stat is incremented
#[tokio::test]
async fn test_tx_error_stat() {
    let mut world = World::new().await;
    let _ap = world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;

    world.when_global_stats_are_captured().await;
    world.when_chip_transmits_data_to_slirp(0).await;
    world.then_frame_errors_increased_by(1).await;
}

// Scenario: Normal tx packets
// Given a Wifi Medium with a mocked gateway
// When packets route to AP and Slirp
// Then the respective _tx counters are incremented successfully
#[tokio::test]
async fn test_hostapd_and_network_tx_stats() {
    let mut world = World::new_with_gateway(Box::new(crate::world::MockGateway::new())).await;
    world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;

    world.when_global_stats_are_captured().await;

    world.when_chip_transmits_mgmt_to_ap(0).await;
    world.then_hostapd_frames_tx_increased_by(1).await;

    world.when_chip_transmits_data_to_slirp(0).await;
    world.then_network_packets_tx_increased_by(1).await;
}
