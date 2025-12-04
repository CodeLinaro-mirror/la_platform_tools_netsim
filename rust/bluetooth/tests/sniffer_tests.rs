// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils;
use crate::test_utils::mock_sink;
use ::bluetooth::server::Server;
use netsim_api::chips::{
    BeaconParams, BluetoothMode, BluetoothParams, ChipId, CreateChipParams, NetworkParams,
    SnifferParams,
};
use netsim_proto::model::chip::BleBeacon;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_sniffer_receives_advertisement() {
    test_utils::setup_logging();
    let (server, client) = Server::new();
    tokio::spawn(async move {
        server.run().await;
    });

    // 1. Create a beacon.
    let id = ChipId(1);
    let create_chip_params = CreateChipParams {
        name: "beacon".to_string(),
        manufacturer: "test".to_string(),
        product_name: "test".to_string(),
        network_params: NetworkParams::Bluetooth(BluetoothParams {
            address: "00:11:22:3D:44:55".to_string(),
            bt_properties: Default::default(),
            mode: BluetoothMode::Beacon(BeaconParams { ble_beacon: BleBeacon::default() }),
        }),
        id,
        packet_stream: None,
        packet_sink: None,
    };
    client.create_chip(create_chip_params).await;

    // 2. Create a sniffer with the mock packet sink.
    let (sink, mut sink_rx) = mock_sink();
    let id = ChipId(2);
    let create_chip_params = CreateChipParams {
        name: "sniffer".to_string(),
        manufacturer: "test".to_string(),
        product_name: "test".to_string(),
        network_params: NetworkParams::Bluetooth(BluetoothParams {
            address: "".to_string(),
            bt_properties: Default::default(),
            mode: BluetoothMode::Sniffer(SnifferParams {}),
        }),
        id,
        packet_stream: None,
        packet_sink: Some(sink),
    };
    client.create_chip(create_chip_params).await;

    // 3. Wait for the advertisement packet.
    timeout(Duration::from_secs(1), async {
        sink_rx.recv().await.unwrap();
    })
    .await
    .expect("Did not receive advertisement within 1 second");
}
