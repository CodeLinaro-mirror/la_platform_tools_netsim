// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils::{self, TestFixture};
use netsim_api::chips::{BeaconParams, BluetoothMode, ChipId, CreateParams};
use netsim_proto::model::chip::BleBeacon;

/// Tests that a chip can be successfully added to the server.
#[tokio::test]
async fn test_add_chip() {
    let TestFixture { client, _server_task } = test_utils::setup();

    let chip_id = ChipId(1);
    let create_chip_params = CreateParams {
        id: chip_id,
        packet_stream: None,
        packet_sink: None,
        config: test_utils::create_chip_config(BluetoothMode::Beacon(BeaconParams {
            ble_beacon: BleBeacon::default(),
        })),
    };
    client.create(create_chip_params).await.expect("creating chip");

    let chip_count: usize = client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // TODO: Implement ChipRequest::GetChip().
}
