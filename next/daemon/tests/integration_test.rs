// Copyright 2023-2025 The Android Open Source Project

use crate::world::World;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use netsim_model::initial_info::{ChipInfo, ChipKind};
use packet_stream::{Streams, TransportType};
use std::time::Duration;
use tokio::time::timeout;

// HACK: Raw HCI packets until rootcanal packet crate is easily usable
const HCI_RESET_COMMAND: [u8; 3] = [0x03, 0x0c, 0x00]; // OpCode, Length

// Feature: Device Integration via UDS
//
//   As a virtual device
//   I want to connect to netsim via Unix Domain Socket
//   So that I can send and receive radio packets

// Scenario: Bluetooth HCI Reset via UDS
//   Given a running Netsim Daemon listening on UDS
//   When a client connects and identifies as a Bluetooth chip
//   And the client sends an HCI Reset command
//   Then the client receives an HCI Command Complete event
#[tokio::test]
async fn test_bluetooth_hci_reset() {
    // Given a running Netsim Daemon
    let mut world = World::new().await;

    // Capture UDS path before spawning daemon (which consumes the daemon instance)
    let uds_path = world
        .daemon
        .as_ref()
        .expect("Daemon not present")
        .uds_path()
        .expect("No UDS path")
        .to_path_buf();
    let uds_path_str = uds_path.to_str().expect("Invalid UDS path").to_string();

    // Start daemon
    let daemon_task = world.spawn_daemon();

    // Spawn the client task
    let client_task = tokio::spawn(async move {
        // Allow some time for netsimd to fully start
        tokio::time::sleep(Duration::from_millis(100)).await;

        let transport = TransportType::uds(uds_path_str);
        let chip_info = ChipInfo::new("bt_test", ChipKind::BLUETOOTH);

        let streams = Streams::new();
        let (mut stream, mut sink) = streams
            .connect(transport, chip_info)
            .await
            .expect("Failed to connect to netsim socket");

        // Send HCI Reset Command
        sink.send(Bytes::from_static(&HCI_RESET_COMMAND)).await.expect("Failed to send HCI Reset");

        // Receive and check response
        match timeout(Duration::from_secs(2), stream.next()).await {
            Ok(Some(Ok(packet))) => {
                assert!(packet.len() >= 6, "Response too short");
                let expected_event: [u8; 6] = [0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
                assert_eq!(&packet[0..6], expected_event, "HCI Reset event did not match");
            }
            Ok(Some(Err(e))) => panic!("Failed to receive response: {}", e),
            Ok(None) => panic!("Stream closed unexpectedly"),
            Err(_) => panic!("Timeout waiting for HCI Reset response"),
        }
    });

    // Wait for client
    match timeout(Duration::from_secs(5), async move {
        let _ = client_task.await;
    })
    .await
    {
        Ok(_) => {}
        Err(_) => panic!("Test timed out"),
    }
}
