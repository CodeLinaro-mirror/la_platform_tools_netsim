// Copyright 2023-2025 The Android Open Source Project

use bytes::Bytes;
use daemon::netsimd::NetsimDaemon;
use futures::{SinkExt, StreamExt};
use netsim_api::initial_info::{ChipInfo, ChipKind};
use packet_stream::{Streams, TransportType};
use std::time::Duration;
use tokio::time::timeout;

// HACK: Raw HCI packets until rootcanal packet crate is easily usable
const HCI_RESET_COMMAND: [u8; 3] = [0x03, 0x0c, 0x00]; // OpCode, Length

#[tokio::test]
async fn test_bluetooth_hci_reset() {
    test_bluetooth_hci_reset_internal().await
}

async fn test_bluetooth_hci_reset_internal() {
    // Run netsimd in the background
    let (daemon, _ini_file) = NetsimDaemon::new().await.expect("Failed to create daemon");
    let uds_path = daemon.uds_path().expect("NetsimDaemon has no UDS path");
    let netsimd_handle = tokio::spawn(daemon.run());

    // Allow some time for netsimd to start and create the socket
    tokio::time::sleep(Duration::from_millis(500)).await;

    let uds_path_str = uds_path.to_str().expect("Invalid UDS path");

    let transport = TransportType::uds(uds_path_str.to_string());
    let chip_info = ChipInfo::new("bt_test", ChipKind::BLUETOOTH);

    let streams = Streams::new();
    let (mut stream, mut sink) =
        streams.connect(transport, chip_info).await.expect("Failed to connect to netsim socket");

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

    netsimd_handle.abort();
}
