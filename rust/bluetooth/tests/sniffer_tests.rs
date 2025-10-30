// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils::{self, mock_sink, TestFixture};
use netsim_api::chips::{BeaconParams, BluetoothMode, ChipId, CreateParams, SnifferParams};
use netsim_proto::model::chip::BleBeacon;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_sniffer_receives_advertisement() {
    let TestFixture { client, _server_task } = test_utils::setup();

    // 1. Create a beacon.
    let id = ChipId(1);
    let create_chip_params = CreateParams {
        id,
        packet_stream: None,
        packet_sink: None,
        config: test_utils::create_chip_config(BluetoothMode::Beacon(BeaconParams {
            ble_beacon: BleBeacon::default(),
        })),
    };
    client.create(create_chip_params).await.ok();

    // 2. Create a sniffer with the mock packet sink.
    let (sink, mut sink_rx) = mock_sink();
    let id = ChipId(2);
    let create_chip_params = CreateParams {
        id,
        packet_stream: None,
        packet_sink: Some(sink),
        config: test_utils::create_chip_config(BluetoothMode::Sniffer(SnifferParams {})),
    };
    client.create(create_chip_params).await.ok();

    // 3. Wait for the advertisement packet.
    timeout(Duration::from_secs(1), async {
        sink_rx.recv().await.unwrap();
    })
    .await
    .expect("Did not receive advertisement within 1 second");
}
