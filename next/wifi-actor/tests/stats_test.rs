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

// Scenario: Throughput bytes window propagation
// Given a Wifi Medium with a mocked gateway
// When a chip transmits data, mock time advances 5.1s, and transmits again
// Then the max_upload_throughput is verified to be greater than 0
#[tokio::test]
async fn test_throughput_stats() {
    let mut world = World::new_with_gateway(Box::new(crate::world::MockGateway::new())).await;
    world.given_an_ap().await;
    let _sender = world.given_a_chip(1).await;

    // Ensure the Dummy packet from `given_a_chip` has already been processed with
    // clock=0
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // First transmission opens the Throughput window using current system clock
    let current_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    world.mock_clock.store(current_millis);
    world.when_chip_transmits_data_to_slirp(0).await;

    // Ensure the first packet is fully processed at `current_millis`
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Subvert the 5-second `tokio::time::sleep` by artificially bumping time
    // forward 5050ms
    world.mock_clock.store(current_millis + 5050);

    // Second transmission immediately flushes the previous window and calculates
    // max_throughput
    world.when_chip_transmits_data_to_slirp(0).await;

    // Wait minimal time for the async channel dispatch
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Verify
    world.then_max_upload_throughput_is_greater_than(0.0).await;
}
