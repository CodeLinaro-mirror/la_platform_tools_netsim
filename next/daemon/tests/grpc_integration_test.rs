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
    let (daemon, _ini_guard) = setup_daemon().await;

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
    let (daemon, _ini_guard) = setup_daemon().await;

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

#[tokio::test]
async fn test_patch_device_resolution() {
    let (daemon, _ini_guard) = setup_daemon().await;
    let grpc_port = daemon.grpc_port().expect("NetsimDaemon has no gRPC port");

    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let env = Arc::new(EnvBuilder::new().build());
        let ch = ChannelBuilder::new(env).connect(&format!("127.0.0.1:{}", grpc_port));
        let client = FrontendServiceClient::new(ch);

        // 1. Create Device "Resolution-Device"
        let mut create_req = CreateDeviceRequest::new();
        let mut device_create = DeviceCreate::new();
        device_create.name = "Resolution-Device".to_string();
        // Add a dummy chip to satisfy creation requirements
        let mut beacon = ChipCreate::new();
        beacon.name = "beacon".to_string();
        beacon.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH_BEACON);
        device_create.chips.push(beacon);

        create_req.device = MessageField::some(device_create);
        let create_resp = client
            .create_device_async(&create_req)
            .expect("Failed create")
            .await
            .expect("RPC failed");
        let device_id = create_resp.device.id;
        assert!(device_id > 0);

        // 2. Patch Device by Name (ID = 0/None)
        let mut patch_req = netsim_proto::frontend::PatchDeviceRequest::new();
        let mut patch_fields =
            netsim_proto::frontend::patch_device_request::PatchDeviceFields::new();
        patch_fields.name = Some("Resolution-Device".to_string());
        patch_fields.position = MessageField::some(netsim_proto::model::Position {
            x: 10.0,
            y: 10.0,
            z: 0.0,
            ..Default::default()
        });
        patch_req.device = MessageField::some(patch_fields);

        client
            .patch_device_async(&patch_req)
            .expect("Failed patch name")
            .await
            .expect("RPC failed");

        // Verify position update (Name patch)
        let list_resp = client
            .list_device_async(&netsim_proto::empty::Empty::new())
            .expect("List failed")
            .await
            .expect("RPC failed");
        let device = list_resp
            .devices
            .iter()
            .find(|d| d.name == "Resolution-Device")
            .expect("Device missing");
        let pos = device.position.as_ref().unwrap();
        assert!((pos.x - 10.0).abs() < 0.001);

        // 3. Patch Device by Explicit ID
        let mut patch_req_id = netsim_proto::frontend::PatchDeviceRequest::new();
        patch_req_id.id = Some(device_id);
        let mut patch_fields_id =
            netsim_proto::frontend::patch_device_request::PatchDeviceFields::new();
        patch_fields_id.position = MessageField::some(netsim_proto::model::Position {
            x: 20.0,
            y: 20.0,
            z: 0.0,
            ..Default::default()
        });
        patch_req_id.device = MessageField::some(patch_fields_id);
        client
            .patch_device_async(&patch_req_id)
            .expect("Failed patch ID")
            .await
            .expect("RPC failed");

        // Verify position update (ID patch)
        let list_resp_final = client
            .list_device_async(&netsim_proto::empty::Empty::new())
            .expect("List failed")
            .await
            .expect("RPC failed");
        let device_id_patch =
            list_resp_final.devices.iter().find(|d| d.id == device_id).expect("Device missing");
        let pos_id = device_id_patch.position.as_ref().unwrap();
        assert!((pos_id.x - 20.0).abs() < 0.001);
    });

    let daemon_task = daemon.run_daemon();
    match timeout(Duration::from_secs(10), async move {
        tokio::select! { _ = client_task => {}, _ = daemon_task => {} }
    })
    .await
    {
        Ok(_) => {}
        Err(_) => panic!("Test timed out"),
    }
}

#[tokio::test]
async fn test_chip_updates() {
    let (daemon, _ini_guard) = setup_daemon().await;
    let grpc_port = daemon.grpc_port().expect("NetsimDaemon has no gRPC port");

    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let env = Arc::new(EnvBuilder::new().build());
        let ch = ChannelBuilder::new(env).connect(&format!("127.0.0.1:{}", grpc_port));
        let client = FrontendServiceClient::new(ch);

        // 1. Create Device with UWB, WiFi, and Bluetooth
        let mut create_req = CreateDeviceRequest::new();
        let mut device_create = DeviceCreate::new();
        device_create.name = "Chip-Update-Device".to_string();

        let mut uwb_chip = ChipCreate::new();
        uwb_chip.name = "uwb0".to_string();
        uwb_chip.kind = EnumOrUnknown::new(ChipKind::UWB);
        device_create.chips.push(uwb_chip);

        let mut wifi_chip = ChipCreate::new();
        wifi_chip.name = "wifi0".to_string();
        wifi_chip.kind = EnumOrUnknown::new(ChipKind::WIFI);
        device_create.chips.push(wifi_chip);

        let mut bt_chip = ChipCreate::new();
        bt_chip.name = "bt0".to_string();
        bt_chip.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH);
        bt_chip.address = "00:11:22:33:44:55".to_string(); // Requires valid address
        device_create.chips.push(bt_chip);

        create_req.device = MessageField::some(device_create);
        let create_resp = client
            .create_device_async(&create_req)
            .expect("Failed create")
            .await
            .expect("RPC failed");
        let device_id = create_resp.device.id;
        assert!(device_id > 0);

        // 2. Patch Device Position (should trigger updates on all chips)
        let mut patch_req = netsim_proto::frontend::PatchDeviceRequest::new();
        patch_req.id = Some(device_id);
        let mut patch_fields =
            netsim_proto::frontend::patch_device_request::PatchDeviceFields::new();
        patch_fields.position = MessageField::some(netsim_proto::model::Position {
            x: 50.0,
            y: 50.0,
            z: 0.0,
            ..Default::default()
        });
        patch_req.device = MessageField::some(patch_fields);

        match client.patch_device_async(&patch_req).expect("Failed patch").await {
            Ok(_) => {}
            Err(e) => panic!("Patch failed (likely due to missing update handler): {}", e),
        }

        // Verify position update reflected in device list
        let list_resp = client
            .list_device_async(&netsim_proto::empty::Empty::new())
            .expect("List failed")
            .await
            .expect("RPC failed");
        let device = list_resp.devices.iter().find(|d| d.id == device_id).expect("Device missing");
        let pos = device.position.as_ref().unwrap();
        assert!((pos.x - 50.0).abs() < 0.001);
    });

    let daemon_task = daemon.run_daemon();
    match timeout(Duration::from_secs(10), async move {
        tokio::select! { _ = client_task => {}, _ = daemon_task => {} }
    })
    .await
    {
        Ok(_) => {}
        Err(_) => panic!("Test timed out"),
    }
}

async fn setup_daemon() -> (NetsimDaemon, daemon::ini_file::IniFileGuard) {
    let temp_dir = std::env::temp_dir().join(format!("netsim_test_{}", rand::random::<u32>()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Setup netsimd with isolated directories
    let mut args = daemon::args::Args::default();
    args.logtostderr = true; // Disable log redirection to avoid segfaults in tests
    let startup_mode = NetsimDaemon::new_with_dirs(temp_dir.clone(), temp_dir.clone(), args)
        .await
        .expect("Failed to create daemon");
    match startup_mode {
        StartUpMode::Owner(daemon, ini_guard) => (daemon, ini_guard),
        _ => panic!("Expected to start as Owner"),
    }
}
