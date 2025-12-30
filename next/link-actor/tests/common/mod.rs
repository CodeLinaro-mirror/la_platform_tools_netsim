// Copyright 2025 The Android Open Source Project

use client::LinkClient;
use link_actor::LinkActor;
use netsim_model::chip::{ChipClient, ChipId, ChipKind};
use std::collections::HashMap;

pub struct TestFixture {
    pub _actor_task: tokio::task::JoinHandle<()>,
    pub client: LinkClient,
    pub mock_chip_controller: Option<actor_framework::mock::MockClient<client::MockChipEntity>>,
    pub mock_chip_client: Option<client::MockChipClient>,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        self._actor_task.abort();
    }
}

pub async fn setup() -> TestFixture {
    let mut clients = HashMap::new();
    let (mock_controller, mock_client) = client::MockChipClient::new();
    clients.insert(ChipKind::BLUETOOTH, Box::new(mock_client.clone()) as Box<dyn ChipClient>);
    clients.insert(ChipKind::WIFI, Box::new(mock_client.clone()) as Box<dyn ChipClient>);
    clients.insert(ChipKind::UWB, Box::new(mock_client.clone()) as Box<dyn ChipClient>);

    let mut fixture = setup_actor(clients).await;
    fixture.mock_chip_controller = Some(mock_controller);
    fixture.mock_chip_client = Some(mock_client);

    // Initial context with some chips for testing using Global Actions
    fixture
        .client
        .action(None, link_api::LinkAction::NotifyChipAdded(ChipId(1), ChipKind::BLUETOOTH))
        .await
        .unwrap();
    fixture
        .client
        .action(None, link_api::LinkAction::NotifyChipAdded(ChipId(2), ChipKind::BLUETOOTH))
        .await
        .unwrap();
    fixture
        .client
        .action(None, link_api::LinkAction::NotifyChipAdded(ChipId(3), ChipKind::WIFI))
        .await
        .unwrap();

    fixture
}

pub async fn setup_actor(clients: HashMap<ChipKind, Box<dyn ChipClient>>) -> TestFixture {
    let (actor, client) = link_actor::new();
    let link_actor_state = LinkActor::new(clients);
    let actor_task = tokio::spawn(actor.run(link_actor_state));

    TestFixture {
        _actor_task: actor_task,
        client,
        mock_chip_controller: None,
        mock_chip_client: None,
    }
}
