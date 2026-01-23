// Copyright 2023-2025 The Android Open Source Project

use crate::test_utils::{self, TestFixture};
use netsim_model::chip::{BeaconParams, BleBeacon, BluetoothMode, ChipClient, ChipCreate, ChipId};
use netsim_model::device::DeviceId;

/// Tests that a chip can be successfully added to the actor.
#[tokio::test]
async fn test_add_chip() {
    test_add_chip_inner().await;
}

async fn test_add_chip_inner() {
    let TestFixture { client, _actor_task } = test_utils::setup();

    let chip_id = ChipId(1);
    let create_chip_params = ChipCreate {
        id: chip_id,
        packet_stream: None,
        packet_sink: None,
        config: test_utils::create_chip_config(BluetoothMode::Beacon(Box::new(BeaconParams {
            ble_beacon: BleBeacon::default(),
        }))),
        device_id: DeviceId(1),
    };
    client.create(create_chip_params).await.expect("creating chip");

    let chip_count: usize = client.read_count_for_testing().await.expect("chip count");
    assert_eq!(chip_count, 1);

    // TODO: Implement ChipRequest::GetChip().
}
