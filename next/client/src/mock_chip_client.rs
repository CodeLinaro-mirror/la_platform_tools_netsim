// Copyright 2025 The Android Open Source Project

use actor_framework::mock::MockClient;
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use netsim_model::chip::{Chip, ChipClient, ChipCreate, ChipId, ChipUpdate};
use netsim_model::client_error::ClientError;
use netsim_model::stats::NetsimRadioStats;

/// A mock entity to satisfy the ActorService trait for MockClient.
#[derive(Clone, Debug)]
pub struct MockChipEntity;

#[derive(Debug, Clone)]
pub enum MockChipAction {
    GetStatistics,
    Reset { id: ChipId },
}

#[derive(Debug, Clone)]
pub enum MockChipActionResult {
    Statistics(Vec<NetsimRadioStats>),
    None,
}

#[derive(Debug, thiserror::Error)]
#[error("MockChipError")]
pub struct MockChipError;

#[async_trait]
impl ActorService for MockChipEntity {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = MockChipAction;
    type ActionResult = MockChipActionResult;
    type Error = MockChipError;
    type Entity = Chip;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        _params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        Ok(id.unwrap_or(ChipId(0)))
    }

    async fn handle_get(
        &self,
        _id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(None)
    }

    async fn handle_update(
        &mut self,
        _id: Self::Id,
        _update: Self::Update,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        Ok(Chip::default())
    }

    async fn handle_delete(
        &mut self,
        _id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            MockChipAction::GetStatistics => Ok(MockChipActionResult::Statistics(vec![])),
            MockChipAction::Reset { .. } => Ok(MockChipActionResult::None),
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(vec![])
    }
}

/// A mock chip client that wraps the framework's MockClient.
#[derive(Clone)]
pub struct MockChipClient {
    client: actor_framework::ResourceClient<MockChipEntity>,
    pub updates: std::sync::Arc<std::sync::Mutex<Vec<(ChipId, ChipUpdate)>>>,
}

impl Default for MockChipClient {
    fn default() -> Self {
        Self::new().1
    }
}

impl MockChipClient {
    /// Creates a new mock client and controller.
    ///
    /// Returns a tuple of `(MockClient<MockChipEntity>, MockChipClient)`:
    /// 1. `MockClient<MockChipEntity>` (The Controller): Use this to SET EXPECTATIONS.
    /// 2. `MockChipClient` (The Client): Pass this to your application.
    pub fn new() -> (MockClient<MockChipEntity>, Self) {
        let mock = MockClient::new();
        let client = Self {
            client: mock.client(),
            updates: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        };
        (mock, client)
    }
}

#[async_trait]
impl ChipClient for MockChipClient {
    async fn create(&self, params: ChipCreate) -> Result<(), ClientError> {
        self.client.create(params).await.map(|_| ()).map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.client
            .get(id)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.updates.lock().unwrap().push((id, patch.clone()));
        self.client.update(id, patch).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.client.delete(id).await.map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn read_statistics(&self) -> Result<Vec<NetsimRadioStats>, ClientError> {
        match self.client.perform_action(None, MockChipAction::GetStatistics).await {
            Ok(MockChipActionResult::Statistics(stats)) => Ok(stats),
            Ok(_) => Err(ClientError::Recv("Unexpected action result".into())),
            Err(e) => Err(ClientError::Send(e.to_string())),
        }
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.client
            .list()
            .await
            .map(|chips| chips.len())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        Ok(())
    }

    async fn reset(&self, id: ChipId) -> Result<(), ClientError> {
        self.client
            .perform_action(Some(id), MockChipAction::Reset { id })
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
