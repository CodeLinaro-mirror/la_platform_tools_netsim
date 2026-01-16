// Copyright 2026 The Android Open Source Project

use client::DeviceClient;
use futures::SinkExt;
use netsim_model::chip::{Chip, ChipClient, ChipCreate, ChipId, NetworkParams, UwbCreate};
use netsim_model::chip_error::ChipError;
use netsim_model::client_error::ClientError;
use netsim_model::device::DeviceId;
use uwb::{UwbActor, UwbClient};

/// The BDD World for UWB Actor tests.
pub struct World {
    pub client: UwbClient,
    pub _device_client: DeviceClient, // Keep reference if we need to check notifications, or use a Mock
    _actor_task: tokio::task::JoinHandle<()>,
}

impl Drop for World {
    fn drop(&mut self) {
        self._actor_task.abort();
    }
}

impl World {
    pub async fn new() -> Self {
        let (runner, client) = uwb::new();

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
            // Expect clone_box recursively if needed, but UwbActor probably doesn't clone it again?
            // Better to be safe:
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
        let actor_task = tokio::spawn(async move { runner.run(actor).await });

        World { client, _device_client: device_client, _actor_task: actor_task }
    }

    pub fn create_uwb_params(id: u32) -> ChipCreate {
        ChipCreate {
            id: ChipId(id),
            // Use empty stream for testing to avoid SyncStream boilerplate
            packet_stream: Some(Box::new(futures::stream::empty())),
            packet_sink: Some(Box::pin(
                futures::sink::drain()
                    .sink_map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "drain")),
            )),
            config: netsim_model::chip::ChipConfig {
                name: format!("uwb_chip_{}", id),
                manufacturer: "Netsim".to_string(),
                product_name: "TestUwb".to_string(),
                network_params: NetworkParams::Uwb(UwbCreate {}),
            },
            device_id: DeviceId(1),
        }
    }

    pub async fn when_create_chip(&self, chip_id: u32) -> Result<(), ChipError> {
        let params = Self::create_uwb_params(chip_id);
        self.client.create(params).await.map_err(|e| match e {
            ClientError::Chip(err) => err,
            _ => ChipError::Internal(e.to_string()),
        })
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
}
