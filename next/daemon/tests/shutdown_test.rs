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
    // 1. Start Daemon with 1s timeout
    let mut args = daemon::args::Args::default();
    args.logtostderr = true;
    args.idle_shutdown_timeout = Some(1000);
    let mut world = World::new_with_args(args).await;
    let daemon_task = world.spawn_daemon();

    // 2. Connect Client (mimic emulator)
    let client = world.ensure_packet_client();
    let (mut client_sender, _client_receiver) =
        client.stream_packets().expect("Failed to create stream");

    // 3. Add Chip
    println!("Sending InitialInfo with chip configuration...");
    let mut initial_req = PacketRequest::new();
    let mut chip_info = ChipInfo::new();

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

    // 5. Verify EARLY Check (should NOT be shut down yet)
    // Timeout is 1s. We wait 0.5s. Daemon should still be running.
    println!("Verifying daemon is still running at 0.5s...");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    if daemon_task.is_finished() {
        panic!("Daemon shut down too early! Timeout was 1s, but finished in <0.5s");
    }

    // 6. Wait for shutdown (should happen after 1s total)
    println!("Waiting for daemon shutdown...");
    // We already waited 0.5s. The idle timeout is 1s.
    // It should shut down at 1.0s. We allow up to 1.1s (100ms margin).
    // So we wait an specific 1000ms more to be safe.
    let result = tokio::time::timeout(std::time::Duration::from_millis(1000), daemon_task).await;

    // 7. Assert Shutdown
    match result {
        Ok(Ok(_)) => println!("Daemon shut down successfully"),
        Ok(Err(e)) => panic!("Daemon task failed: {}", e),
        Err(_) => panic!("Daemon failed to shut down within timeout (took > 1.5s)"),
    }
}

// Scenario: Daemon does NOT shut down before the timeout
//   Given a running Netsim Daemon with 2s timeout
//   When a client disconnects
//   Then the daemon is still running after 1s
//   And the daemon shuts down after 2.5s
#[tokio::test]
async fn test_daemon_stays_alive_before_timeout() {
    // 1. Start Daemon with 2s idle timeout
    let mut args = daemon::args::Args::default();
    args.idle_shutdown_timeout = Some(2000);
    args.logtostderr = true;
    let mut world = World::new_with_args(args).await;
    let daemon_task = world.spawn_daemon();

    // 2. Connect Client
    println!("Connecting client...");
    let client = world.ensure_packet_client();
    let (mut client_sender, _client_receiver) =
        client.stream_packets().expect("Failed to create stream");

    // 3. Add Chip and Verify
    println!("Adding chip...");
    let mut initial_req = PacketRequest::new();
    let mut chip_info = ChipInfo::new();
    let mut chip = netsim_proto::startup::Chip::new();
    chip.kind =
        netsim_proto::protobuf::EnumOrUnknown::new(netsim_proto::common::ChipKind::BLUETOOTH);
    chip.address = "33:33:33:33:33:33".to_string();
    chip_info.chip = netsim_proto::protobuf::MessageField::some(chip);
    initial_req.set_initial_info(chip_info);
    client_sender
        .send((initial_req, grpcio::WriteFlags::default()))
        .await
        .expect("Failed to send InitialInfo");

    // Wait for chip to be added
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 4. Disconnect
    println!("Disconnecting client...");
    drop(client_sender);

    // 5. Wait 1s (daemon should still be alive)
    println!("Waiting 1s (should be alive)...");
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    if daemon_task.is_finished() {
        panic!("Daemon shut down prematurely!");
    }

    // 6. Wait another 2s (total 3s vs 2s timeout) -> should be dead
    println!("Waiting 2s for shutdown...");
    let result = tokio::time::timeout(std::time::Duration::from_millis(2000), daemon_task).await;
    match result {
        Ok(Ok(_)) => println!("Daemon shut down successfully after timeout"),
        Ok(Err(e)) => panic!("Daemon task failed: {}", e),
        Err(_) => panic!("Daemon failed to shut down within timeout"),
    }
}
