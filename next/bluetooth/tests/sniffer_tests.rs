// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils::{self, mock_sink, TestFixture};
use netsim_model::chip::{
    BeaconParams, BleBeacon, BluetoothMode, ChipClient, ChipCreate, ChipId, SnifferParams,
};
use netsim_model::device::DeviceId;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_sniffer_receives_advertisement() {
    test_sniffer_receives_advertisement_inner().await;
}

async fn test_sniffer_receives_advertisement_inner() {
    let TestFixture { client, _actor_task } = test_utils::setup();

    // 1. Create a beacon.
    let id = ChipId(1);
    let create_chip_params = ChipCreate {
        id,
        packet_stream: None,
        packet_sink: None,
        config: test_utils::create_chip_config(BluetoothMode::Beacon(Box::new(BeaconParams {
            ble_beacon: BleBeacon::default(),
        }))),
        device_id: DeviceId(1),
    };
    client.create(create_chip_params).await.ok();

    // 2. Create a sniffer with the mock packet sink.
    let (sink, mut sink_rx) = mock_sink();
    let id = ChipId(2);
    let create_chip_params = ChipCreate {
        id,
        packet_stream: None,
        packet_sink: Some(sink),
        config: test_utils::create_chip_config(BluetoothMode::Sniffer(SnifferParams {})),
        device_id: DeviceId(1),
    };
    client.create(create_chip_params).await.ok();

    // 3. Wait for the advertisement packet.
    timeout(Duration::from_secs(1), async {
        sink_rx.recv().await.unwrap();
    })
    .await
    .expect("Did not receive advertisement within 1 second");
}
