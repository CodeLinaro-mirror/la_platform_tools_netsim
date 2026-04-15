// Copyright 2026 Google LLC

use crate::world::TestWorld;

#[tokio::test]
async fn test_websocket_happy_path() {
    let mut world = TestWorld::new().await;

    let client_idx = world
        .given_a_websocket_client_connected(Some("test-peer"), Some("00:11:22:33:44:55"))
        .await;

    world.then_backend_receives_add_chip_request(client_idx, "test-peer", "00:11:22:33:44:55");

    let test_packet = vec![0x01, 0x02, 0x03, 0x04];
    world.when_client_sends_packet(client_idx, &test_packet).await;
    world.then_backend_receives_packet(client_idx, &test_packet).await;

    let response_packet = vec![0x05, 0x06, 0x07, 0x08];
    world.when_backend_sends_packet(client_idx, &response_packet).await;
    world.then_client_receives_packet(client_idx, &response_packet).await;

    world.when_client_closes_connection(client_idx).await;
    world.then_backend_deletes_chip(client_idx).await;
}
