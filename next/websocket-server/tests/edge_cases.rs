// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::world::TestWorld;

#[tokio::test]
async fn test_default_query_parameters() {
    let mut world = TestWorld::new().await;

    // Connect with no query parameters
    let client_idx = world.given_a_websocket_client_connected(None, None).await;

    world.then_backend_receives_default_add_chip_request(client_idx);
}

#[tokio::test]
async fn test_invalid_resource_path() {
    let mut world = TestWorld::new().await;

    world.when_client_attempts_to_connect_to_invalid_path().await;
    world.then_the_connection_fails_with_status(404);
}

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let mut world = TestWorld::new().await;

    let client_a = world.given_a_websocket_client_connected(Some("client-a"), None).await;
    let client_b = world.given_a_websocket_client_connected(Some("client-b"), None).await;

    let packet_a = [0xAA];
    let packet_b = [0xBB];

    world.when_client_sends_packet(client_a, &packet_a).await;
    world.when_client_sends_packet(client_b, &packet_b).await;

    world.then_backend_receives_packet(client_a, &packet_a).await;
    world.then_backend_receives_packet(client_b, &packet_b).await;

    world.when_client_closes_connection(client_a).await;
    world.then_backend_deletes_chip(client_a).await;

    // Client B should still be alive
    world.when_backend_sends_packet(client_b, &packet_b).await;
    world.then_client_receives_packet(client_b, &packet_b).await;
}

#[tokio::test]
async fn test_backend_initiated_termination() {
    let mut world = TestWorld::new().await;

    let client_idx = world.given_a_websocket_client_connected(None, None).await;

    // Terminate from backend
    world.when_backend_closes_connection(client_idx).await;

    // Wait for cleanup
    world.then_backend_deletes_chip(client_idx).await;

    // Verify client receives EOF/Close
    world.then_client_receives_eof(client_idx).await;
}

#[tokio::test]
async fn test_large_data_payloads() {
    let mut world = TestWorld::new().await;
    let client_idx = world.given_a_websocket_client_connected(None, None).await;

    let large_packet = [0xCC; 2000];

    world.when_client_sends_packet(client_idx, &large_packet).await;
    world.then_backend_receives_packet(client_idx, &large_packet).await;

    world.when_backend_sends_packet(client_idx, &large_packet).await;
    world.then_client_receives_packet(client_idx, &large_packet).await;
}

#[tokio::test]
async fn test_ipv6_connectivity() {
    let mut world = TestWorld::new().await;

    let client_idx = world.given_an_ipv6_websocket_client_connected(None, None).await;

    let test_packet = [0x01, 0x02];
    world.when_client_sends_packet(client_idx, &test_packet).await;
    world.then_backend_receives_packet(client_idx, &test_packet).await;
}
