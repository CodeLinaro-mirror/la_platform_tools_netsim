// Copyright 2025 The Android Open Source Project

use client::LinkClient;
use link_actor::LinkActor;
use netsim_model::chip::{ChipId, ChipKind};

pub struct TestFixture {
    pub _actor_task: tokio::task::JoinHandle<()>,
    pub client: LinkClient,
    // Add mock radio channels if needed
}

pub async fn setup() -> TestFixture {
    let (actor, resource_client) = link_actor::new();
    let client = LinkClient::new(resource_client);

    // Run the actor with empty state
    let link_actor_state = LinkActor::new();
    let actor_task = tokio::spawn(actor.run(link_actor_state));

    // Initial context with some chips for testing using Global Actions
    client
        .action(None, link_api::LinkAction::NotifyChipAdded(ChipId(1), ChipKind::BLUETOOTH))
        .await
        .unwrap();
    client
        .action(None, link_api::LinkAction::NotifyChipAdded(ChipId(2), ChipKind::BLUETOOTH))
        .await
        .unwrap();
    client
        .action(None, link_api::LinkAction::NotifyChipAdded(ChipId(3), ChipKind::WIFI))
        .await
        .unwrap();

    TestFixture { _actor_task: actor_task, client }
}
