// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils::{mock_sink, mock_stream};
use crate::world::{self, World};
use bytes::Bytes;
use log::info;
use netsim_model::chip::{BluetoothMode, ChipClient, ChipCreate, DeviceParams};
use tokio::time::{timeout, Duration};

// Feature: Bluetooth Chip Lifecycle
//
//   As a client
//   I want to manage bluetooth chips
//   So that I can simulate bluetooth devices behavior

// Scenario: Perform HCI Reset
//
//   Given a bluetooth chip in device mode
//   When an HCI reset command is sent
//   Then the chip responds with Command Complete
#[tokio::test]
async fn test_hci_reset_command() {
    let mut world = World::new();

    let (stream, stream_tx) = mock_stream();
    let (sink, mut sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = world.next_chip_id();
    let create_chip_config = ChipCreate {
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
        config: world::World::create_chip_config(id, BluetoothMode::Device(DeviceParams {})),
        device_id: world.device_id,
    };
    world.when_create_chip(create_chip_config).await.expect("creating chip");

    // 2. Send an HCI Reset command.
    let hci_reset_cmd = Bytes::from(vec![0x01, 0x03, 0x0c, 0x00]);
    stream_tx.send(hci_reset_cmd).await.map_err(|e| info!("err:{:?}", e.0)).expect("sending");

    // 3. Wait for the HCI Command Complete event.
    let response = timeout(Duration::from_secs(1), sink_rx.recv()).await.unwrap().unwrap();

    // 4. Verify the response.
    // Expected: Command Complete for Reset, status OK.
    let expected_response = vec![0x04, 0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
    assert_eq!(response, expected_response);
}

// Scenario: Chip removal on packet stream error
//
//   Given a bluetooth chip
//   When the packet stream is closed unexpectedly
//   Then the chip is removed from the actor
#[tokio::test]
async fn test_chip_dies_on_packet_stream_error() {
    let mut world = World::new();

    let (stream, stream_tx) = mock_stream();
    let (sink, _sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = world.next_chip_id();
    let create_chip_spec = ChipCreate {
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
        config: world::World::create_chip_config(id, BluetoothMode::Device(DeviceParams {})),
        device_id: world.device_id,
    };
    world.when_create_chip(create_chip_spec).await.expect("creating chip");

    // A small delay to ensure the chip is registered before we check the count.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let chip_count: usize = world.client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // 2. Trigger a packet stream error by closing the channel.
    drop(stream_tx);

    // 4. Verify the chip has been removed.
    // A small delay is needed to ensure the actor has time to process the death notice.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let chip_count: usize = world.client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 0);
}

// Scenario: Chip removal on explicit delete
//
//   Given a bluetooth chip
//   When a delete command is sent
//   Then the chip is removed from the actor
#[tokio::test]
async fn test_delete_chip_shuts_down_task() {
    let mut world = World::new();

    let (stream, mut _stream_tx) = mock_stream();
    let (sink, mut _sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = world.next_chip_id();
    let create_chip_params = ChipCreate {
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
        config: world::World::create_chip_config(id, BluetoothMode::Device(DeviceParams {})),
        device_id: world.device_id,
    };
    world.when_create_chip(create_chip_params).await.expect("creating chip");

    let chip_count: usize = world.client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // 2. Send a DeleteChip command.
    world.when_delete_chip(id).await.expect("delete chip");

    // 4. Verify the chip has been removed.
    let chip_count: usize = world.client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 0);
}

// Scenario: Chip removal on packet sink error
//
//   Given a bluetooth chip
//   When the packet sink is closed unexpectedly
//   Then the chip is removed from the actor
#[tokio::test]
async fn test_chip_dies_on_packet_sink_error() {
    let mut world = World::new();

    let (stream, stream_tx) = mock_stream();
    let (sink, sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = world.next_chip_id();
    let create_chip_spec = ChipCreate {
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
        config: world::World::create_chip_config(id, BluetoothMode::Device(DeviceParams {})),
        device_id: world.device_id,
    };
    world.when_create_chip(create_chip_spec).await.expect("creating chip");

    // A small delay to ensure the chip is registered before we check the count.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let chip_count: usize = world.client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // 2. Trigger a packet sink error by closing the receiver.
    drop(sink_rx);

    // 3. Send a packet to trigger the sink write (which will fail).
    // We send an HCI Reset command.
    let hci_reset_cmd = Bytes::from(vec![0x01, 0x03, 0x0c, 0x00]);
    stream_tx.send(hci_reset_cmd).await.expect("sending hci cmd");

    // 4. Verify the chip has been removed.
    // A small delay is needed to ensure the actor has time to process the death notice.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let chip_count: usize = world.client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 0);
}
