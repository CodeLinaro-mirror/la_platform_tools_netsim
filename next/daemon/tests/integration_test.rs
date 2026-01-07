// Copyright 2023-2025 The Android Open Source Project

use bytes::Bytes;
use daemon::netsimd::{NetsimDaemon, StartUpMode};
use futures::{SinkExt, StreamExt};
use netsim_model::initial_info::{ChipInfo, ChipKind};
use packet_stream::{Streams, TransportType};
use std::time::Duration;
use tokio::time::timeout;

// HACK: Raw HCI packets until rootcanal packet crate is easily usable
const HCI_RESET_COMMAND: [u8; 3] = [0x03, 0x0c, 0x00]; // OpCode, Length

#[tokio::test]
async fn test_bluetooth_hci_reset() {
    let temp_dir = std::env::temp_dir().join(format!("netsim_test_{}", rand::random::<u32>()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Setup netsimd
    let mut args = daemon::args::Args::default();
    args.logtostderr = true; // Disable log redirection to avoid segfaults in tests
    let startup_mode = NetsimDaemon::new_with_dirs(temp_dir.clone(), temp_dir.clone(), args)
        .await
        .expect("Failed to create daemon");
    let (daemon, _ini_guard) = match startup_mode {
        StartUpMode::Owner(daemon, ini_guard) => (daemon, ini_guard),
        _ => panic!("Expected to start as Owner"),
    };
    let uds_path = daemon.uds_path().expect("NetsimDaemon has no UDS path");

    // Spawn the client task
    let client_task = tokio::spawn(async move {
        // Allow some time for netsimd to fully start (although listeners are up)
        tokio::time::sleep(Duration::from_millis(100)).await;

        let uds_path_str = uds_path.to_str().expect("Invalid UDS path");
        let transport = TransportType::uds(uds_path_str.to_string());
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

    // Run the daemon in the current task
    let daemon_task = daemon.run_daemon();

    // Wait for the client to finish, with a timeout for the whole test
    match timeout(Duration::from_secs(5), async move {
        tokio::select! {
            _ = client_task => {},
            _ = daemon_task => {},
        }
    })
    .await
    {
        Ok(_) => {}
        Err(_) => panic!("Test timed out"),
    }
}
