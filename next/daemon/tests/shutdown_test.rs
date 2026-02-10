// Copyright 2025 The Android Open Source Project

use futures::SinkExt;
use netsim_proto::{packet_streamer::PacketRequest, startup::ChipInfo};

use crate::world::World;

// Scenario: Daemon shuts down when the last client disconnects
//   Given a running Netsim Daemon
//   When a client connects, adds a chip, and then disconnects
//   Then the daemon counts 0 active chips and shuts down
#[tokio::test]
async fn test_auto_shutdown_on_chip_removal() {
    // 1. Start Daemon
    let mut world = World::new().await;
    let daemon_task = world.spawn_daemon();

    // 2. Connect Client (mimic emulator)
    let client = world.ensure_packet_client();
    let (mut client_sender, _client_receiver) =
        client.stream_packets().expect("Failed to create stream");

    // 3. Add Chip
    println!("Sending InitialInfo with chip configuration...");
    let mut initial_req = PacketRequest::new();
    let mut chip_info = ChipInfo::new();
    // name is deprecated but we can set it for completeness or use device_info if
    // needed chip_info.name = "shutdown-test-chip".to_string(); // 'name' in
    // ChipInfo is deprecated

    // Create a valid Chip model (startup::Chip)
    let mut chip = netsim_proto::startup::Chip::new();
    chip.kind =
        netsim_proto::protobuf::EnumOrUnknown::new(netsim_proto::common::ChipKind::BLUETOOTH);
    chip.address = "33:33:33:33:33:33".to_string();
    chip.manufacturer = "TestMfg".to_string();
    chip.product_name = "TestProduct".to_string();

    chip_info.chip = netsim_proto::protobuf::MessageField::some(chip);

    let mut device_info = netsim_proto::startup::DeviceInfo::new();
    device_info.name = "shutdown-test-device".to_string();
    device_info.kind = "EMULATOR".to_string();
    chip_info.device_info = netsim_proto::protobuf::MessageField::some(device_info);

    initial_req.set_initial_info(chip_info);

    client_sender
        .send((initial_req, grpcio::WriteFlags::default()))
        .await
        .expect("Failed to send InitialInfo");

    println!("InitialInfo sent. Stream established.");

    // Allow some time for the daemon to process the new chip
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // 3.5 Verify Device List
    println!("Verifying device list...");
    let devices = world.when_list_devices().await;
    println!("Found devices: {:?}", devices);
    assert!(!devices.is_empty(), "Device list should not be empty");
    let device =
        devices.iter().find(|d| d.name == "shutdown-test-device").expect("Test device not found");
    assert!(!device.chips.is_empty(), "Test device should have chips");
    println!("Device verification successful: Found {} with chips", device.name);

    // 4. Disconnect (Drop sender)
    println!("Dropping client sender to trigger disconnect...");
    drop(client_sender);

    // 5. Wait for shutdown
    println!("Waiting for daemon shutdown...");
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), daemon_task).await;

    // 6. Assert Shutdown
    match result {
        Ok(Ok(_)) => println!("Daemon shut down successfully"),
        Ok(Err(e)) => panic!("Daemon task failed: {}", e),
        Err(_) => panic!("Daemon failed to shut down within timeout"),
    }
}
