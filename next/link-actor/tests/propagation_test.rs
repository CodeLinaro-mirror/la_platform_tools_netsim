use link_api::{LinkAction, LinkCreate};
use netsim_model::chip::{ChipClient, ChipId, ChipKind};
use std::collections::HashMap;

use crate::common::setup_actor;

#[tokio::test]
async fn test_link_propagation() {
    let mut mock_client = netsim_model::chip::MockChipClient::new();

    mock_client
        .expect_update()
        .withf(|id, patch| {
            // Verify updates contain links
            *id == ChipId(1)
                && patch.links.is_some()
                && patch.links.as_ref().unwrap().contains(&(ChipId(2), -50))
        })
        .times(1)
        .returning(|_, _| Ok(netsim_model::chip::Chip::default()));

    let mut clients = HashMap::new();
    clients.insert(ChipKind::BLUETOOTH, Box::new(mock_client) as Box<dyn ChipClient>);

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

    // actor_task is dropped and aborted automatically when TestFixture is dropped
    // Mock verification happens on drop.
}
