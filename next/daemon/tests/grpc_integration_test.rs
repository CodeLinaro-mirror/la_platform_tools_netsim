// Copyright 2023-2025 The Android Open Source Project

use crate::world::World;
use futures::{SinkExt, StreamExt};
use netsim_proto::common::ChipKind;
use netsim_proto::hci_packet::hcipacket::PacketType;
use netsim_proto::model::ChipCreate;
use netsim_proto::protobuf::{EnumOrUnknown, MessageField};

// Feature: gRPC Frontend Lifecycle
//
//   As a developer
//   I want to manage devices and streams via gRPC
//   So that I can control the netsim environment

// Scenario: Verify Daemon Version and Device Lifecycle
//   Given a running Netsim Daemon
//   Then I can retrieve the daemon version
//   When I create a new device
//   Then the device appears in the device list
//   When I delete the device
//   Then the device disappears from the device list
#[tokio::test]
async fn test_grpc_frontend_lifecycle() {
    // Given a running Netsim Daemon
    let mut world = World::new().await;

    // Spawn the daemon in a separate task
    let daemon_task = world.spawn_daemon();

    // Verify Version
    let version = world.when_get_version().await;
    assert_eq!(version, "0.0.1-next");

    // When I create a new device
    let device_name = "grpc-test-device";
    let device_id = world.when_create_device(device_name, "beacon-chip").await;
    assert!(device_id > 0);

    // Then the device list contains the new device
    let devices = world.when_list_devices().await;
    assert!(devices.iter().any(|d| d.id == device_id && d.name == device_name));

    // When I delete the chip (device)
    world.when_delete_chip(device_id).await;

    // Then the device list does not contain the device
    let devices_after = world.when_list_devices().await;
    assert!(!devices_after.iter().any(|d| d.id == device_id));

    // Cleanup: In a real scenario we might want to shut down gracefully,
    // but here we just let the test finish which aborts the daemon task.
    // However, to be clean we could abort or just let it drop.
    // The TempDir will be cleaned up on Drop of World.
}

// Scenario: Packet Streamer Lifecycle
//   Given a running Netsim Daemon
//   When I establish a packet stream
//   And I send initial chip info
//   And I send a packet
//   Then the stream remains open and processes packets
#[tokio::test]
async fn test_packet_streamer_lifecycle() {
    // Given a running Netsim Daemon
    let mut world = World::new().await;

    // Spawn the daemon
    let daemon_task = world.spawn_daemon();

    // Spawn client task to interact with packet streamer
    // We need to clone world bits or just do it inline here since we own world.
    // However, `spawn_daemon` consumed the daemon instance from world.
    // The client is created via gRPC port which we have in world.

    let client = world.ensure_packet_client();
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
}

// Scenario: Patch Device Resolution
//   Given a running Netsim Daemon
//   When I create a device named "Resolution-Device"
//   And I patch the device position by name
//   Then the device position is updated
//   When I patch the device position by ID
//   Then the device position is updated again
#[tokio::test]
async fn test_patch_device_resolution() {
    let mut world = World::new().await;
    let _daemon_task = world.spawn_daemon();

    // 1. Create Device "Resolution-Device"
    let mut beacon = ChipCreate::new();
    beacon.name = "beacon".to_string();
    beacon.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH_BEACON);
    beacon.address = "11:22:33:44:55:66".to_string();
    let mut ble_beacon = netsim_proto::model::chip_create::BleBeaconCreate::new();
    ble_beacon.address = "11:22:33:44:55:66".to_string();
    beacon.set_ble_beacon(ble_beacon);

    let device_id = world.when_create_device_with_chips("Resolution-Device", vec![beacon]).await;
    assert!(device_id > 0);

    // 2. Patch Device by Name (ID = 0/None)
    let mut patch_req = netsim_proto::frontend::PatchDeviceRequest::new();
    let mut patch_fields = netsim_proto::frontend::patch_device_request::PatchDeviceFields::new();
    patch_fields.name = Some("Resolution-Device".to_string());
    patch_fields.position = MessageField::some(netsim_proto::model::Position {
        x: 10.0,
        y: 10.0,
        z: 0.0,
        ..Default::default()
    });
    patch_req.device = MessageField::some(patch_fields);

    world.when_patch_device(&patch_req).await;

    // Verify position update (Name patch)
    let devices = world.when_list_devices().await;
    let device = devices.iter().find(|d| d.name == "Resolution-Device").expect("Device missing");
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

    world.when_patch_device(&patch_req_id).await;

    // Verify position update (ID patch)
    let devices_final = world.when_list_devices().await;
    let device_id_patch = devices_final.iter().find(|d| d.id == device_id).expect("Device missing");
    let pos_id = device_id_patch.position.as_ref().unwrap();
    assert!((pos_id.x - 20.0).abs() < 0.001);
}

// Scenario: Chip Update
//   Given a running Netsim Daemon
//   When I create a device with a Bluetooth Beacon chip
//   And I patch the device position (which triggers a chip update)
//   Then the device position is updated
#[tokio::test]
async fn test_chip_update() {
    let mut world = World::new().await;
    let _daemon_task = world.spawn_daemon();

    // 1. Create Device with 1 Bluetooth Beacon chip
    let mut beacon = ChipCreate::new();
    beacon.name = "beacon0".to_string();
    beacon.kind = EnumOrUnknown::new(ChipKind::BLUETOOTH_BEACON);
    let mut ble_beacon = netsim_proto::model::chip_create::BleBeaconCreate::new();
    ble_beacon.address = "11:22:33:44:55:66".to_string();
    beacon.set_ble_beacon(ble_beacon);

    let device_id = world.when_create_device_with_chips("Chip-Update-Device", vec![beacon]).await;
    assert!(device_id > 0);

    // 2. Patch Device Position (should trigger updates on the chip)
    let mut patch_req = netsim_proto::frontend::PatchDeviceRequest::new();
    patch_req.id = Some(device_id);
    let mut patch_fields = netsim_proto::frontend::patch_device_request::PatchDeviceFields::new();
    patch_fields.position = MessageField::some(netsim_proto::model::Position {
        x: 50.0,
        y: 50.0,
        z: 0.0,
        ..Default::default()
    });
    patch_req.device = MessageField::some(patch_fields);

    world.when_patch_device(&patch_req).await;

    // Verify position update reflected in device list
    let devices = world.when_list_devices().await;
    let device = devices.iter().find(|d| d.id == device_id).expect("Device missing");
    let pos = device.position.as_ref().unwrap();
    assert!((pos.x - 50.0).abs() < 0.001);
}

// Scenario: Radio State Propagation
//   Given a running Netsim Daemon
//   When I create a device with a Bluetooth chip
//   Then the initial radio state is TRUE (default)
//   When I patch the device to turn the radio OFF
//   Then the radio state becomes FALSE
#[tokio::test]
async fn test_radio_state_propagation() {
    // Given a running Netsim Daemon
    let mut world = World::new().await;
    let _daemon_task = world.spawn_daemon();

    // When I create a device with a Bluetooth chip
    let bt_chip = World::make_bluetooth_chip("bt0", "00:11:22:33:44:55");
    let device_id = world.when_create_device_with_chips("Radio-Test-Device", vec![bt_chip]).await;

    // Resolve Chip ID
    let devices = world.when_list_devices().await;
    let device = devices.iter().find(|d| d.id == device_id).expect("Device missing");
    let chip_id = device.chips[0].id;

    // Then the initial radio state is TRUE (default)
    world.then_radio_state_is(device_id, ChipKind::BLUETOOTH, true).await;

    // When I patch the device to turn the radio OFF
    world.when_patch_state(device_id, Some(chip_id), ChipKind::BLUETOOTH, false).await;

    // Then the radio state becomes FALSE
    world.then_radio_state_is(device_id, ChipKind::BLUETOOTH, false).await;
}

#[tokio::test]
async fn test_chip_update_resolution_by_variant() {
    let mut world = World::new().await;
    let _daemon_task = world.spawn_daemon();

    // 1. Create Device with multiple chips
    let bt_chip = World::make_bluetooth_chip("bt-res", "00:11:22:33:44:55");
    let uwb_chip = World::make_uwb_chip("uwb-res");

    let device_id =
        world.when_create_device_with_chips("Resolution-By-Variant", vec![bt_chip, uwb_chip]).await;

    // 2. Patch Bluetooth Radio State *WITHOUT* Chip ID
    world.when_patch_state(device_id, None, ChipKind::BLUETOOTH, false).await;

    // 3. Verify
    world.then_radio_state_is(device_id, ChipKind::BLUETOOTH, false).await;
}
