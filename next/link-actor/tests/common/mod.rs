// Copyright 2025 The Android Open Source Project

use client::LinkClient;
use link_actor::LinkActor;
use netsim_model::chip::{ChipClient, ChipId, ChipKind};
use std::collections::HashMap;

pub struct TestFixture {
    pub _actor_task: tokio::task::JoinHandle<()>,
    pub client: LinkClient,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        self._actor_task.abort();
    }
}

pub async fn setup() -> TestFixture {
    let mut clients = HashMap::new();
    let _mock_client = create_default_mock();
    // automated mocks are not cloneable by default in a way that shares expectations,
    // so we create independent mocks for each slot.
    // For standard tests using setup(), we just need them to reply Ok.
    clients.insert(ChipKind::BLUETOOTH, Box::new(create_default_mock()) as Box<dyn ChipClient>);
    clients.insert(ChipKind::WIFI, Box::new(create_default_mock()) as Box<dyn ChipClient>);
    clients.insert(ChipKind::UWB, Box::new(create_default_mock()) as Box<dyn ChipClient>);

    let fixture = setup_actor(clients).await;

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

fn create_default_mock() -> netsim_model::chip::MockChipClient {
    let mut mock = netsim_model::chip::MockChipClient::new();
    mock.expect_read().returning(|_| Ok(netsim_model::chip::Chip::default()));
    mock.expect_update().returning(|_, _| Ok(netsim_model::chip::Chip::default()));
    mock.expect_create().returning(|_| Ok(()));
    mock.expect_delete().returning(|_| Ok(()));
    mock.expect_read_statistics().returning(|| Ok(vec![]));
    mock.expect_reset().returning(|_| Ok(()));
    mock.expect_clone_box().returning(|| Box::new(create_default_mock()));
    mock
}

pub async fn setup_actor(clients: HashMap<ChipKind, Box<dyn ChipClient>>) -> TestFixture {
    let (actor, client) = link_actor::new();
    let link_actor_state = LinkActor::new(clients);
    let actor_task = tokio::spawn(actor.run(link_actor_state));

    TestFixture { _actor_task: actor_task, client }
}
