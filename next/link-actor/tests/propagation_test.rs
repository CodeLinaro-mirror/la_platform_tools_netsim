use async_trait::async_trait;
use client::LinkClient;
use link_actor::LinkActor;
use link_api::{LinkAction, LinkCreate};
use netsim_model::chip::{Chip, ChipClient, ChipCreate, ChipId, ChipKind, ChipUpdate};
use netsim_model::client_error::ClientError;
use netsim_model::stats::NetsimRadioStats;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MockChipClient {
    updates: Arc<Mutex<Vec<(ChipId, ChipUpdate)>>>,
}

impl MockChipClient {
    fn new() -> Self {
        Self { updates: Arc::new(Mutex::new(Vec::new())) }
    }
}

#[async_trait]
impl ChipClient for MockChipClient {
    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.updates.lock().unwrap().push((id, patch));
        Ok(Chip::default())
    }
    async fn create(&self, _params: ChipCreate) -> Result<(), ClientError> {
        Ok(())
    }
    async fn read(&self, _id: ChipId) -> Result<Chip, ClientError> {
        Ok(Chip::default())
    }
    async fn delete(&self, _id: ChipId) -> Result<(), ClientError> {
        Ok(())
    }
    async fn shutdown(&self) -> Result<(), ClientError> {
        Ok(())
    }
    async fn reset(&self, _id: ChipId) -> Result<(), ClientError> {
        Ok(())
    }
    async fn read_statistics(&self) -> Result<Vec<NetsimRadioStats>, ClientError> {
        Ok(vec![])
    }
    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        Ok(0)
    }
}

#[tokio::test]
async fn test_link_propagation() {
    let mut actor_state = LinkActor::new();
    let client = MockChipClient::new();

    let mut clients = HashMap::new();
    clients.insert(ChipKind::BLUETOOTH, Box::new(client.clone()) as Box<dyn ChipClient>);
    actor_state.set_chip_clients(clients);

    let (actor, resource_client) = link_actor::new();
    let actor_task = tokio::spawn(actor.run(actor_state));

    let link_client = LinkClient::new(resource_client);

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

    actor_task.abort();
}
