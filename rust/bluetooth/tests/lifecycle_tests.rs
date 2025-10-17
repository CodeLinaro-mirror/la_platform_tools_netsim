// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils;
use crate::test_utils::{mock_sink, mock_stream};
use ::bluetooth::server::Server;
use bytes::Bytes;
use log::info;
use netsim_api::chips::{
    BluetoothMode, BluetoothParams, ChipId, CreateParams, DeviceParams, NetworkParams,
};
use netsim_proto::configuration::Controller as RootcanalController;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_hci_reset_command() {
    test_utils::setup_logging();

    let (server, client) = Server::new();
    tokio::spawn(async move {
        server.run().await;
    });

    let (stream, mut stream_tx) = mock_stream();
    let (sink, mut sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = ChipId(1);

    let create_chip_params = CreateParams {
        name: "test_chip".to_string(),
        manufacturer: "test_manufacturer".to_string(),
        product_name: "test_product".to_string(),
        network_params: NetworkParams::Bluetooth(BluetoothParams {
            address: "AB:CD:EF:11:22:33".to_string(),
            bt_properties: RootcanalController::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
    };
    client.create(create_chip_params).await.expect("creating chip");
    // 2. Send an HCI Reset command.
    let hci_reset_cmd = Bytes::from(vec![0x03, 0x0c, 0x00]);
    stream_tx.send(hci_reset_cmd).await.map_err(|e| info!("err:{:?}", e.0)).expect("sending");

    // 3. Wait for the HCI Command Complete event.
    let response = timeout(Duration::from_secs(1), sink_rx.recv()).await.unwrap().unwrap();

    // 4. Verify the response.
    // Expected: Command Complete for Reset, status OK.
    let expected_response = vec![0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
    assert_eq!(response, expected_response);
}

#[tokio::test]
async fn test_chip_dies_on_packet_stream_error() {
    test_utils::setup_logging();

    let (server, client) = Server::new();
    tokio::spawn(async move {
        server.run().await;
    });

    let (stream, mut stream_tx) = mock_stream();
    let (sink, mut sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = ChipId(1);
    let create_chip_params = CreateParams {
        name: "test_chip".to_string(),
        manufacturer: "test_manufacturer".to_string(),
        product_name: "test_product".to_string(),
        network_params: NetworkParams::Bluetooth(BluetoothParams {
            address: "BE:EF:FA:CE:11:22".to_string(),
            bt_properties: RootcanalController::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
    };
    client.create(create_chip_params).await.expect("creating chip");

    // A small delay to ensure the chip is registered before we check the count.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let chip_count: usize = client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // 2. Trigger a packet stream error by closing the channel.
    drop(stream_tx);

    // 4. Verify the chip has been removed.
    // A small delay is needed to ensure the server has time to process the death notice.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let chip_count: usize = client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 0);
}

#[tokio::test]
async fn test_delete_chip_shuts_down_task() {
    test_utils::setup_logging();

    let (server, client) = Server::new();
    tokio::spawn(async move {
        server.run().await;
    });

    let (stream, mut stream_tx) = mock_stream();
    let (sink, mut sink_rx) = mock_sink();

    // 1. Create a virtual device chip.
    let id = ChipId(1);
    let create_chip_params = CreateParams {
        name: "test_chip".to_string(),
        manufacturer: "test_manufacturer".to_string(),
        product_name: "test_product".to_string(),
        network_params: NetworkParams::Bluetooth(BluetoothParams {
            address: "DE:AD:BE:EF:33:44".to_string(),
            bt_properties: RootcanalController::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        id,
        packet_stream: Some(stream),
        packet_sink: Some(sink),
    };
    client.create(create_chip_params).await.expect("creating chip");

    let chip_count: usize = client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // 2. Send a DeleteChip command.
    client.delete(id).await.expect("delete chip");

    // 4. Verify the chip has been removed.
    let chip_count: usize = client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 0);
}
