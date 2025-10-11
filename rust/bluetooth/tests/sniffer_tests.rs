// Copyright 2023-2025 The Android Open Source Project

use crate::utils;
use ::bluetooth::manager::{BluetoothCommand, BluetoothManager};
use bytes::Bytes;
use netsim_api::{
    BeaconCreationParams, BluetoothSnifferParams, ChipParams, CreateChipParams, PacketStreamerApi,
};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};

#[derive(Default, Clone)]
struct MockPacketStreamer {
    packets: Arc<Mutex<Vec<Bytes>>>,
}

#[async_trait::async_trait]
impl PacketStreamerApi for MockPacketStreamer {
    async fn read_packet(&mut self) -> Result<Option<Vec<u8>>, netsim_api::Error> {
        Ok(None)
    }

    async fn write_packet(&mut self, packet: Vec<u8>) -> Result<(), netsim_api::Error> {
        self.packets.lock().unwrap().push(Bytes::from(packet));
        Ok(())
    }
}

#[tokio::test]
async fn test_sniffer_receives_advertisement() {
    utils::setup_logging();
    let (mut bt_manager, command_tx) = BluetoothManager::new();
    tokio::spawn(async move {
        bt_manager.run().await;
    });

    let mock_streamer = MockPacketStreamer::default();

    // 1. Create a beacon.
    let beacon_params =
        BeaconCreationParams { address: "00:11:22:3D:44:55".to_string(), ..Default::default() };
    let chip_params = ChipParams::BluetoothBeacon(beacon_params);
    let (responder, rx) = oneshot::channel();
    let create_chip_params =
        CreateChipParams { chip_params, packet_streamer: Box::new(MockPacketStreamer::default()) };
    command_tx
        .send(BluetoothCommand::CreateChip { params: create_chip_params, responder })
        .await
        .unwrap();
    rx.await.unwrap().unwrap();

    // 2. Create a sniffer.
    let sniffer_params = BluetoothSnifferParams::default();
    let chip_params = ChipParams::BluetoothSniffer(sniffer_params);
    let (responder, rx) = oneshot::channel();
    let create_chip_params =
        CreateChipParams { chip_params, packet_streamer: Box::new(mock_streamer.clone()) };
    command_tx
        .send(BluetoothCommand::CreateChip { params: create_chip_params, responder })
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
