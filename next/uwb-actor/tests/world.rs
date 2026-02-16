// Copyright 2026 The Android Open Source Project

use std::collections::HashMap;

use bytes::Bytes;
use client::DeviceClient;
use netsim_model::{
    chip::{Chip, ChipClient, ChipCreate, ChipId, ChipKindParams, UwbCreate},
    chip_error::ChipError,
    client_error::ClientError,
    device::DeviceId,
};
use netsim_testing::mocks::{mock_sink, mock_stream};
use tokio::sync::mpsc::{Receiver, Sender};
use uwb_actor::{UwbActor, UwbClient};

/// The BDD World for UWB Actor tests.
pub struct World {
    pub client: UwbClient,
    pub _device_client: DeviceClient, /* Keep reference if we need to check notifications, or
                                       * use a Mock */
    pub packet_txs: HashMap<ChipId, Sender<Bytes>>,
    pub packet_rxs: HashMap<ChipId, Receiver<Vec<u8>>>,
    _actor_task: tokio::task::JoinHandle<()>,
}

impl Drop for World {
    fn drop(&mut self) {
        self._actor_task.abort();
    }
}

impl World {
    pub async fn new() -> Self {
        let (runner, client) = uwb_actor::new();

        // Use MockActorClient for the DeviceClient
        let mut mock_device_client = actor_framework::MockActorClient::new();
        mock_device_client
            .expect_perform_action()
            .returning(|_, _| Ok(device_api::DeviceActionResult::Success));
        mock_device_client.expect_clone_box().returning(|| {
            let mut new_mock = actor_framework::MockActorClient::new();
            new_mock
                .expect_perform_action()
                .returning(|_, _| Ok(device_api::DeviceActionResult::Success));
            // Expect clone_box recursively if needed, but UwbActor probably doesn't clone
            // it again? Better to be safe:
            new_mock.expect_clone_box().returning(|| {
                let mut inner_mock = actor_framework::MockActorClient::new();
                inner_mock
                    .expect_perform_action()
                    .returning(|_, _| Ok(device_api::DeviceActionResult::Success));
                inner_mock.expect_clone_box().returning(|| {
                    let mut deep_mock = actor_framework::MockActorClient::new();
                    deep_mock
                        .expect_perform_action()
                        .returning(|_, _| Ok(device_api::DeviceActionResult::Success));
                    // Hope we don't need deeper than this (Mock4)
                    Box::new(deep_mock)
                });
                Box::new(inner_mock)
            });
            Box::new(new_mock)
        });

        let device_client = DeviceClient::new(Box::new(mock_device_client));

        // Spawn actor
        let actor = UwbActor::new(device_client.clone());
        let actor_task = tokio::spawn(runner.run(actor));

        World {
            client,
            _device_client: device_client,
            packet_txs: HashMap::new(),
            packet_rxs: HashMap::new(),
            _actor_task: actor_task,
        }
    }

    pub async fn when_create_chip(&mut self, chip_id: u32) -> Result<(), ChipError> {
        let id = ChipId(chip_id);
        let (stream, packet_tx) = mock_stream();
        let (sink, packet_rx) = mock_sink();
        let params = ChipCreate {
            id,
            packet_stream: Some(stream),
            packet_sink: Some(sink),
            config: netsim_model::chip::ChipConfig {
                name: format!("uwb_chip_{id}"),
                manufacturer: "Netsim".to_string(),
                product_name: "TestUwb".to_string(),
                chip_kind_params: ChipKindParams::Uwb(UwbCreate {}),
            },
            device_id: DeviceId(1),
        };

        self.client.create(params).await.map_err(|e| match e {
            ClientError::Chip(err) => err,
            _ => ChipError::Internal(e.to_string()),
        })?;

        self.packet_txs.insert(id, packet_tx);
        self.packet_rxs.insert(id, packet_rx);

        Ok(())
    }

    pub async fn when_delete_chip(&self, chip_id: u32) -> Result<(), ChipError> {
        self.client.delete(ChipId(chip_id)).await.map_err(|e| match e {
            ClientError::Chip(err) => err,
            _ => ChipError::Internal(e.to_string()),
        })
    }

    pub async fn when_get_chip(&self, chip_id: u32) -> Result<Chip, ChipError> {
        self.client.read(ChipId(chip_id)).await.map_err(|e| match e {
            ClientError::Chip(err) => err,
            _ => ChipError::Internal(e.to_string()),
        })
    }

    pub fn and_packet_stream_is_closed(&mut self, chip_id: u32) {
        let chip_id = ChipId(chip_id);
        self.packet_txs.remove(&chip_id);
    }

    pub fn and_packet_sink_is_closed(&mut self, chip_id: u32) {
        let chip_id = ChipId(chip_id);
        self.packet_rxs.remove(&chip_id);
    }

    pub async fn and_tick_occurs(&mut self) {
        tokio::time::sleep(2 * UwbActor::TICK_INTERVAL).await;
    }

    pub async fn then_chip_does_not_exist(&self, chip_id: u32) {
        // Yield to allow the actor to process the stream/sink closure.
        tokio::task::yield_now().await;
        let result = self.when_get_chip(chip_id).await;
        assert_eq!(result, Err(ChipError::ChipNotFound(ChipId(chip_id))));
    }

    pub async fn when_packet_is_sent(&mut self, chip_id: u32, packet: &[u8]) {
        let chip_id = ChipId(chip_id);
        let tx = self.packet_txs.get_mut(&chip_id).expect("Chip not found or already closed");
        tx.send(Bytes::copy_from_slice(packet)).await.expect("Failed to send packet");
    }

    pub async fn then_packet_is_received(&mut self, chip_id: u32) -> Vec<u8> {
        let chip_id = ChipId(chip_id);
        let rx = self.packet_rxs.get_mut(&chip_id).expect("Chip not found or already closed");
        tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("Timed out waiting for packet")
            .expect("Packet stream closed unexpectedly")
    }

    pub async fn then_chip_exists(&self, chip_id: u32) {
        self.when_get_chip(chip_id).await.expect("Chip should exist");
    }

    pub async fn given_a_chip(&mut self, chip_id: u32) {
        self.when_create_chip(chip_id).await.expect("GIVEN: Failed to create chip");
    }
}
