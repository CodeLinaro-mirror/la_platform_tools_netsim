//! # Generic Client
//!
//! This module defines the generic client for communicating with actors.

use crate::entity::ActorEntity;
use crate::error::FrameworkError;
use crate::message::ResourceRequest;
use tokio::sync::{mpsc, oneshot};

/// A type-safe client for interacting with a `ResourceActor`.
#[derive(Clone)]
/// ## ResourceClient
///
/// The `ResourceClient<T>` provides a type‑safe, async API for interacting with a `ResourceActor<T>`. It forwards CRUD + Action requests over a Tokio mpsc channel and returns results via oneshot channels. The client is cheap to clone and can be shared across tasks.
///
/// * **Cloneable** – holds only a sender, so cloning is inexpensive.
/// * **Async API** – all methods return `Future`s that resolve to `Result<…, FrameworkError>`.
/// * **Generic** – works with any entity that implements `ActorEntity`.
pub struct ResourceClient<T: ActorEntity> {
    sender: mpsc::Sender<ResourceRequest<T>>,
}

impl<T: ActorEntity> ResourceClient<T> {
    pub fn new(sender: mpsc::Sender<ResourceRequest<T>>) -> Self {
        Self { sender }
    }

    pub async fn create(&self, params: T::Create) -> Result<T::Id, FrameworkError> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ResourceRequest::Create { params, respond_to })
            .await
            .map_err(|_| FrameworkError::ActorClosed)?;
        response.await.map_err(|_| FrameworkError::ActorDropped)?
    }

    pub async fn get(&self, id: T::Id) -> Result<Option<T>, FrameworkError> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ResourceRequest::Get { id, respond_to })
            .await
            .map_err(|_| FrameworkError::ActorClosed)?;
        response.await.map_err(|_| FrameworkError::ActorDropped)?
    }

    pub async fn update(&self, id: T::Id, update: T::Update) -> Result<T, FrameworkError> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ResourceRequest::Update { id, update, respond_to })
            .await
            .map_err(|_| FrameworkError::ActorClosed)?;
        response.await.map_err(|_| FrameworkError::ActorDropped)?
    }

    #[allow(dead_code)]
    pub async fn delete(&self, id: T::Id) -> Result<(), FrameworkError> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ResourceRequest::Delete { id, respond_to })
            .await
            .map_err(|_| FrameworkError::ActorClosed)?;
        response.await.map_err(|_| FrameworkError::ActorDropped)?
    }

    pub async fn perform_action(
        &self,
        id: T::Id,
        action: T::Action,
    ) -> Result<T::ActionResult, FrameworkError> {
        let (respond_to, response) = oneshot::channel();
        self.sender
            .send(ResourceRequest::Action { id, action, respond_to })
            .await
            .map_err(|_| FrameworkError::ActorClosed)?;
        response.await.map_err(|_| FrameworkError::ActorDropped)?
    }
}
