// Copyright (C) 2025 The Android Open Source Project

use client::device_client::DeviceClient;
use device_actor::DeviceContext;
use netsim_model::chip::{ChipClient, ChipRequest, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct TestFixture {
    pub actor_task: tokio::task::JoinHandle<()>,
    pub client: DeviceClient,
    pub chip_rx: mpsc::Receiver<ChipRequest>,
}

pub async fn setup() -> TestFixture {
    let (actor, generic_client) = device_actor::new();
    let client = DeviceClient::new(generic_client);

    let (chip_tx, chip_rx) = mpsc::channel(10);
    let chip_client = ChipClient::new(chip_tx);

    let mut chip_clients = HashMap::new();
    chip_clients.insert(NetworkKind::Bluetooth, chip_client);

    let context = DeviceContext {
        chip_clients,
        next_chip_id: Arc::new(AtomicU32::new(0)),
        capture_client: None,
    };

    let actor_task = tokio::spawn(actor.run(context));

    TestFixture { actor_task, client, chip_rx }
}
