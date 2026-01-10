use client::MockChipClient;
use link_api::{LinkAction, LinkCreate};
use netsim_model::chip::{ChipClient, ChipId, ChipKind};
use std::collections::HashMap;

use crate::common::setup_actor;

#[tokio::test]
async fn test_link_propagation() {
    let (_mock_controller, client) = MockChipClient::new();

    let mut clients = HashMap::new();
    clients.insert(ChipKind::BLUETOOTH, Box::new(client.clone()) as Box<dyn ChipClient>);

    let fixture = setup_actor(clients).await;
    let link_client = &fixture.client;

    // Add chips
    link_client
        .action(None, LinkAction::NotifyChipAdded(ChipId(1), ChipKind::BLUETOOTH))
        .await
        .unwrap();
    link_client
        .action(None, LinkAction::NotifyChipAdded(ChipId(2), ChipKind::BLUETOOTH))
        .await
        .unwrap();

    // Create link
    let params = LinkCreate { sender: ChipId(1), receiver: ChipId(2), rssi: -50 };
    let _link_id = link_client.create(params).await.unwrap();

    let updates = client.updates.lock().unwrap();
    assert_eq!(updates.len(), 1, "Should receive update only for sender");

    // Verify updates contain links
    let (id, update) = &updates[0];
    assert_eq!(*id, ChipId(1));
    assert!(update.links.is_some());
    let links = update.links.as_ref().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0], (ChipId(2), -50));

    // actor_task is dropped and aborted automatically when TestFixture is dropped
}
