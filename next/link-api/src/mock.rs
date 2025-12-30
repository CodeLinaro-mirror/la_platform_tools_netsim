// Copyright 2025 The Android Open Source Project

use crate::{Link, LinkAction, LinkClient, LinkCreate, LinkId, LinkUpdate};
use actor_framework::mock::MockClient;
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;

/// A mock entity to satisfy the ActorService trait for MockClient.
#[derive(Clone, Debug)]
pub struct MockLinkEntity;

#[derive(Debug, thiserror::Error)]
#[error("MockLinkError")]
pub struct MockLinkError;

#[async_trait]
impl ActorService for MockLinkEntity {
    type Id = LinkId;
    type Create = LinkCreate;
    type Update = LinkUpdate;
    type Action = LinkAction;
    type ActionResult = ();
    type Error = MockLinkError;
    type Entity = Link;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        _params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        Ok(id.unwrap_or(LinkId(0)))
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
        Ok(Link::default())
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
        _action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(vec![])
    }
}

/// A mock link client that wraps the framework's MockClient.
#[derive(Clone)]
pub struct MockLinkClient {
    client: actor_framework::ResourceClient<MockLinkEntity>,
}

impl Default for MockLinkClient {
    fn default() -> Self {
        Self::new().1
    }
}

impl MockLinkClient {
    /// Creates a new mock client and controller.
    ///
    /// Returns a tuple of `(MockClient<MockLinkEntity>, MockLinkClient)`:
    /// 1. `MockClient<MockLinkEntity>` (The Controller): Use this to SET EXPECTATIONS.
    /// 2. `MockLinkClient` (The Client): Pass this to your application.
    pub fn new() -> (MockClient<MockLinkEntity>, Self) {
        let mock = MockClient::new();
        let client = Self { client: mock.client() };
        (mock, client)
    }
}

#[async_trait]
impl LinkClient for MockLinkClient {
    async fn list(&self) -> Result<Vec<Link>, String> {
        self.client.list().await.map_err(|e| e.to_string())
    }

    async fn create(&self, params: LinkCreate) -> Result<LinkId, String> {
        self.client.create(params).await.map_err(|e| e.to_string())
    }

    async fn update(&self, id: LinkId, patch: LinkUpdate) -> Result<(), String> {
        self.client.update(id, patch).await.map(|_| ()).map_err(|e| e.to_string())
    }

    async fn delete(&self, id: LinkId) -> Result<(), String> {
        self.client.delete(id).await.map_err(|e| e.to_string())
    }

    async fn action(&self, id: Option<LinkId>, action: LinkAction) -> Result<(), String> {
        self.client.perform_action(id, action).await.map_err(|e| e.to_string())
    }
}
