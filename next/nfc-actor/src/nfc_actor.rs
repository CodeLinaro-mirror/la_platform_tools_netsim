// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use device_actor::DeviceClient;
use netsim_model::ChipId;
use tokio::io::{DuplexStream, WriteHalf};
use tracing::info;

pub struct ChipState {
    pub device_id: device_api::DeviceId,
    pub enabled: bool,
    pub casimir_device_id: u16,
    pub nfc_writer: WriteHalf<DuplexStream>,
}

pub struct NfcActor {
    pub device_client: DeviceClient,
    pub active_chips: HashMap<ChipId, ChipState>,
    pub scene_client: Option<crate::scene::SceneClient>,
    pub scene_task: Option<tokio::task::JoinHandle<()>>,
}

impl NfcActor {
    pub fn new(device_client: DeviceClient) -> Self {
        Self { device_client, active_chips: HashMap::new(), scene_client: None, scene_task: None }
    }

    pub fn start_casimir(&mut self) {
        if self.scene_task.is_some() {
            return;
        }
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let scene = crate::scene::NetsimScene::new(cmd_rx);
        let scene_client = crate::scene::SceneClient::new(cmd_tx);

        let scene_task = tokio::spawn(async move {
            scene.await;
        });

        self.scene_task = Some(scene_task);
        self.scene_client = Some(scene_client);
        info!("In-process Casimir scene started");
    }
}

#[derive(Debug)]
pub enum NfcAction {
    Generic(netsim_model::ChipRequest),
    CreateControlChannel {
        respond_to: tokio::sync::oneshot::Sender<
            Result<(tokio::io::DuplexStream, u16), crate::error::NfcError>,
        >,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_casimir() {
        let (_runner, device_client) = device_actor::new();
        let mut actor = NfcActor::new(device_client);
        actor.start_casimir();
        assert!(actor.scene_task.is_some());
        assert!(actor.scene_client.is_some());
    }

    #[tokio::test]
    async fn test_add_device_to_scene() {
        let (_runner, device_client) = device_actor::new();
        let mut actor = NfcActor::new(device_client);
        actor.start_casimir();

        let scene_client = actor.scene_client.as_ref().unwrap().clone();

        // Create a mock duplex stream for the device
        let (_nfc_io, casimir_io) = tokio::io::duplex(1024);

        // Add device to scene
        let add_result = scene_client
            .add_device(move |id, rf_tx| {
                let (rx, tx) = tokio::io::split(casimir_io);
                casimir::Device::nci(id, rx, tx, rf_tx)
            })
            .await;

        assert!(add_result.is_ok());
        let device_id = add_result.unwrap();
        assert_eq!(device_id, 0); // First device should be ID 0 (slot index)
    }
}
