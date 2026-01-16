// Copyright (C) 2025 The Android Open Source Project

use client::device_client::DeviceClient;
use netsim_model::chip::{ChipRequest, LegacyChipClient as ChipClient, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct TestFixture {
    pub client: DeviceClient,
    pub chip_rx: mpsc::Receiver<ChipRequest>,
    pub mock_link_controller: actor_framework::mock::MockClient<link_api::mock::MockLinkEntity>,
    pub mock_link_client: link_api::mock::MockLinkClient,
}

pub async fn setup() -> TestFixture {
    let (chip_tx, chip_rx) = mpsc::channel(10);
    let chip_client = ChipClient::new(chip_tx);

    let mut chip_clients: HashMap<NetworkKind, Box<dyn netsim_model::chip::ChipClient>> =
        HashMap::new();
    chip_clients.insert(NetworkKind::Bluetooth, Box::new(chip_client));

    let (mock_link_controller, mock_link_client) = link_api::mock::MockLinkClient::new();

    let actor_impl = device_actor::new(
        chip_clients,
        Arc::new(AtomicU32::new(0)),
        None,
        Box::new(mock_link_client.clone()),
    );

    let (actor, generic_client) = actor_framework::ResourceActor::new(32);
    let client = DeviceClient::new(generic_client);

    tokio::spawn(actor.run(actor_impl));

    TestFixture { client, chip_rx, mock_link_controller, mock_link_client }
}
