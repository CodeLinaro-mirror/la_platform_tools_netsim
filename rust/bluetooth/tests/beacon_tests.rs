// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils;
use ::bluetooth::server::Server;
use netsim_api::chips::{
    BeaconParams, BluetoothMode, BluetoothParams, ChipId, CreateChipParams, NetworkParams,
};
use netsim_proto::configuration::Controller as RootcanalController;
use netsim_proto::model::chip::BleBeacon;

/// Tests that a chip can be successfully added to the server.
#[tokio::test]
async fn test_add_chip() {
    test_utils::setup_logging();
    let (server, client) = Server::new();
    tokio::spawn(async move {
        server.run().await;
    });

    let chip_id = ChipId(1);
    let create_chip_params = CreateChipParams {
        name: "test_chip".to_string(),
        manufacturer: "test_manufacturer".to_string(),
        product_name: "test_product".to_string(),
        network_params: NetworkParams::Bluetooth(BluetoothParams {
            address: "AB:CD:EF:11:22:33".to_string(),
            bt_properties: RootcanalController::default(),
            mode: BluetoothMode::Beacon(BeaconParams { ble_beacon: BleBeacon::default() }),
        }),
        id: chip_id,
        packet_stream: None,
        packet_sink: None,
    };
    client.create_chip(create_chip_params).await.expect("creating chip");

    let chip_count: usize = client.get_chip_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // TODO: Implement ChipRequest::GetChip().
}
