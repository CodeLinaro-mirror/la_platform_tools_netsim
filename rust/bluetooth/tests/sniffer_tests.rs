// Copyright 2023-2025 The Android Open Source Project

use crate::utils;
use ::bluetooth::manager::BluetoothManager;
use bytes::Bytes;
use netsim_api::chips::{ChipIdentifier, ChipRequest, CreateChipParams};
use netsim_api::packet_streamer::{PacketStreamerApi, PsError};
use netsim_proto::model::chip::BleBeacon;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

#[derive(Default, Clone)]
struct MockPacketStreamer {
    packets: Arc<Mutex<Vec<Bytes>>>,
}

#[async_trait::async_trait]
impl PacketStreamerApi for MockPacketStreamer {
    async fn read_packet(&mut self) -> Result<Option<Vec<u8>>, PsError> {
        Ok(None)
    }

    async fn write_packet(&mut self, packet: Vec<u8>) -> Result<(), PsError> {
        self.packets.lock().unwrap().push(Bytes::from(packet));
        Ok(())
    }
}

#[tokio::test]
async fn test_sniffer_receives_advertisement() {
    utils::setup_logging();
    let (bt_manager, command_tx) = BluetoothManager::new();
    tokio::spawn(async move {
        bt_manager.run().await;
    });

    let mock_streamer = MockPacketStreamer::default();

    // 1. Create a beacon.
    let (respond_to, rx) = oneshot::channel();
    let beacon_id = 1;
    let create_chip_params = CreateChipParams {
        packet_streamer: Box::new(MockPacketStreamer::default()),
        name: "beacon".to_string(),
        manufacturer: "test".to_string(),
        product_name: "test".to_string(),
        network_params: netsim_api::chips::NetworkParams::Bluetooth(
            netsim_api::chips::BluetoothMode::Beacon(netsim_api::chips::BeaconParams {
                address: "00:11:22:3D:44:55".to_string(),
                ble_beacon: BleBeacon::default(),
            }),
        ),
        id: ChipIdentifier(beacon_id),
    };
    command_tx
        .send(ChipRequest::CreateChip { params: create_chip_params, respond_to })
        .await
        .unwrap();
    rx.await.unwrap().unwrap();

    // 2. Create a sniffer.
    let (respond_to, rx) = oneshot::channel();
    let sniffer_id = 2;
    let create_chip_params = CreateChipParams {
        packet_streamer: Box::new(mock_streamer.clone()),
        name: "sniffer".to_string(),
        manufacturer: "test".to_string(),
        product_name: "test".to_string(),
        network_params: netsim_api::chips::NetworkParams::Bluetooth(
            netsim_api::chips::BluetoothMode::Sniffer(netsim_api::chips::SnifferParams {}),
        ),
        id: ChipIdentifier(sniffer_id),
    };
    command_tx
        .send(ChipRequest::CreateChip { params: create_chip_params, respond_to })
        .await
        .unwrap();
    rx.await.unwrap().unwrap();

    // 3. Wait for the advertisement packet.
    timeout(Duration::from_secs(1), async {
        loop {
            if !mock_streamer.packets.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Did not receive advertisement within 1 second");
}
