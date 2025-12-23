// Copyright 2023-2025 The Android Open Source Project

use daemon::netsimd::{NetsimDaemon, StartUpMode};
use futures::{SinkExt, StreamExt};
use grpcio::{ChannelBuilder, EnvBuilder};
use netsim_proto::common::ChipKind;
use netsim_proto::frontend::CreateDeviceRequest;
use netsim_proto::frontend_grpc::FrontendServiceClient;
use netsim_proto::hci_packet::hcipacket::PacketType;
use netsim_proto::model::{ChipCreate, DeviceCreate};
use netsim_proto::protobuf::EnumOrUnknown;
use netsim_proto::protobuf::MessageField;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_grpc_frontend_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!("netsim_test_{}", rand::random::<u32>()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Setup netsimd with isolated directories
    let mut args = daemon::args::Args::default();
    args.logtostderr = true; // Disable log redirection to avoid segfaults in tests
    let startup_mode = NetsimDaemon::new_with_dirs(temp_dir.clone(), temp_dir.clone(), args)
        .await
        .expect("Failed to create daemon");
    let (daemon, _ini_guard) = match startup_mode {
        StartUpMode::Owner(daemon, ini_guard) => (daemon, ini_guard),
        _ => panic!("Expected to start as Owner"),
    };

    // Get gRPC port
    let grpc_port = daemon.grpc_port().expect("NetsimDaemon has no gRPC port");

    // Spawn the client task
    let client_task = tokio::spawn(async move {
        // Allow some time for netsimd to fully start
        tokio::time::sleep(Duration::from_millis(500)).await;

        let env = Arc::new(EnvBuilder::new().build());
        let ch = ChannelBuilder::new(env).connect(&format!("127.0.0.1:{}", grpc_port));
        let client = FrontendServiceClient::new(ch);

        // 1. GetVersion
        let version = client
            .get_version_async(&netsim_proto::empty::Empty::new())
            .expect("GetVersion failed")
            .await
            .expect("RPC failed");
        assert_eq!(version.version, "0.0.1-next");

        // 2. CreateDevice
        let mut create_req = CreateDeviceRequest::new();
        let mut device_create = DeviceCreate::new();
        device_create.name = "grpc-test-device".to_string();

        let mut chip_create = ChipCreate::new();
        chip_create.name = "beacon-chip".to_string();
        chip_create.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH_BEACON);
        chip_create.manufacturer = "TestMfg".to_string();
        chip_create.product_name = "TestProduct".to_string();

        device_create.chips.push(chip_create);
        create_req.device = MessageField::some(device_create);

        let create_resp = client
            .create_device_async(&create_req)
            .expect("CreateDevice failed")
            .await
            .expect("RPC failed");
        let device_id = create_resp.device.id;
        assert!(device_id > 0);
        assert_eq!(create_resp.device.name, "grpc-test-device");

        // 3. ListDevice
        let list_resp = client
            .list_device_async(&netsim_proto::empty::Empty::new())
            .expect("ListDevice failed")
            .await
            .expect("RPC failed");
        assert!(list_resp.devices.iter().any(|d| d.id == device_id));

        // 4. DeleteChip (deletes device)
        let mut delete_req = netsim_proto::frontend::DeleteChipRequest::new();
        delete_req.id = device_id;
        client
            .delete_chip_async(&delete_req)
            .expect("DeleteChip failed")
            .await
            .expect("RPC failed");

        // 5. Verify Deletion
        let list_resp_after = client
            .list_device_async(&netsim_proto::empty::Empty::new())
            .expect("ListDevice failed")
            .await
            .expect("RPC failed");
        assert!(!list_resp_after.devices.iter().any(|d| d.id == device_id));
    });

    // Run the daemon in the current task
    let daemon_task = daemon.run_daemon();

    // Wait for the client to finish
    match timeout(Duration::from_secs(10), async move {
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

#[tokio::test]
async fn test_packet_streamer_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!("netsim_test_{}", rand::random::<u32>()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Setup netsimd with isolated directories
    let mut args = daemon::args::Args::default();
    args.logtostderr = true; // Disable log redirection to avoid segfaults in tests
    let startup_mode = NetsimDaemon::new_with_dirs(temp_dir.clone(), temp_dir.clone(), args)
        .await
        .expect("Failed to create daemon");
    let (daemon, _ini_guard) = match startup_mode {
        StartUpMode::Owner(daemon, ini_guard) => (daemon, ini_guard),
        _ => panic!("Expected to start as Owner"),
    };

    let grpc_port = daemon.grpc_port().expect("NetsimDaemon has no gRPC port");

    // Spawn the client task
    let client_task = tokio::spawn(async move {
        // Allow some time for netsimd to fully start
        tokio::time::sleep(Duration::from_millis(500)).await;

        let env = Arc::new(EnvBuilder::new().build());
        let ch = ChannelBuilder::new(env).connect(&format!("127.0.0.1:{}", grpc_port));
        let client = netsim_proto::packet_streamer_grpc::PacketStreamerClient::new(ch);

        // Create the stream
        let (mut client_sender, mut client_receiver) =
            client.stream_packets().expect("Failed to create stream");

        // 1. Send InitialInfo
        let mut initial_req = netsim_proto::packet_streamer::PacketRequest::new();
        let mut chip_info = netsim_proto::startup::ChipInfo::new();
        chip_info.name = "streamer-test-chip".to_string();
        initial_req.set_initial_info(chip_info);

        client_sender
            .send((initial_req, grpcio::WriteFlags::default()))
            .await
            .expect("Failed to send InitialInfo");

        // 2. Send a Packet (HCI)
        let mut packet_req = netsim_proto::packet_streamer::PacketRequest::new();
        let mut hci_packet = netsim_proto::hci_packet::HCIPacket::new();
        hci_packet.packet_type = PacketType::COMMAND.into();
        hci_packet.packet = vec![0x01, 0x02, 0x03]; // Dummy packet
        packet_req.set_hci_packet(hci_packet);

        client_sender
            .send((packet_req, grpcio::WriteFlags::default()))
            .await
            .expect("Failed to send Packet");

        // 3. Close stream
        client_sender.close().await.expect("Failed to close stream");

        // Verify we don't get an error immediately
        let _ = client_receiver.next().await;
    });

    // Run the daemon
    let daemon_task = daemon.run_daemon();

    // Wait for client
    match timeout(Duration::from_secs(10), async move {
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
