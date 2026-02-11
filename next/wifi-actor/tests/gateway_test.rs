// Copyright 2025 The Android Open Source Project

use crate::world::{MockGateway, World};

#[tokio::test]
async fn test_gateway_routing() {
    // 1. Setup World with MockGateway
    let mock_gateway = MockGateway::new();
    let outgoing_packets = mock_gateway.outgoing_packets.clone();
    let mut world = World::new_with_gateway(Box::new(mock_gateway)).await;

    // 2. Create AP
    world.given_an_ap().await;

    // 3. Create Chip
    let chip1_id = world.given_a_chip(1).await;
    outgoing_packets.lock().unwrap().clear();

    // 3. Chip transmits data destined for Internet (ToDS)
    // This helper sends a ToDS frame destined to a dummy "Gateway" MAC.
    // The WifiActor should route this to the GatewayTrait implementation
    // (MockGateway).
    world.when_chip_transmits_data_to_slirp(0).await;

    // 4. Verify Gateway received the packet
    // Give it a moment to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let packets = outgoing_packets.lock().unwrap();
    assert_eq!(packets.len(), 1, "Gateway should have received 1 packet");

    let (cid, _frame) = &packets[0];
    assert_eq!(cid.0, chip1_id, "Packet should be from Chip 1");

    // We can also verify contents of _frame if needed, but World helper sent
    // known payload.
}
