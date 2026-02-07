// Copyright (C) 2025 The Android Open Source Project

//! Link Client
//!
//! This module provides the [`LinkClient`] struct, which is a type-safe wrapper
//! around the generic [`ResourceClient`]. It provides a convenient API for
//! interacting with Link actors.

use std::ops::Deref;

use actor_framework::ResourceClient;
use netsim_model::chip::{ChipId, ChipKind};

use crate::LinkActor;

/// A client for interacting with the Link Actor.
///
/// This client provides a type-safe API for managing chip-to-chip links.
///
/// # Design Philosophy: Stateless Client
///
/// To keep the implementation lightweight, this client is stateless and
/// non-atomic. It does not internally track the existence of links or combine
/// "check-and-create" operations into a single step.
///
/// # Recommended Usage Pattern
///
/// To ensure a link exists with the correct configuration, consumers should use
/// the **Look-then-Act** approach:
///
/// 1. **Find**: Query `list()` for an existing connection.
/// 2. **Path A (Exists)**: Use `update()` with the existing ID.
/// 3. **Path B (Missing)**: Use `create()` with your parameters.
///
/// # Handling Race Conditions
///
/// In environments where multiple scripts or processes may be acting on the
/// same hardware simultaneously, your "Find" results may become stale before
/// you can "Act."
///
/// If you attempt to `create()` a link that was just created by another
/// process, the server will return an `AlreadyExists` error. Robust
/// applications should catch this error and treat it as a signal to re-fetch
/// the link ID and perform an `update()` instead.
#[derive(Clone, Debug)]
pub struct LinkClient {
    pub(crate) inner: ResourceClient<LinkActor>,
}

impl LinkClient {
    pub fn new(inner: ResourceClient<LinkActor>) -> Self {
        Self { inner }
    }

    /// Helper to find a link ID by sender and receiver.
    ///
    /// This performs a `list()` and linear search.
    /// TODO: Optimize this with a custom action or lookup map in the actor.
    pub async fn get_link_id(
        &self,
        sender: ChipId,
        receiver: ChipId,
    ) -> Result<Option<link_api::LinkId>, actor_framework::FrameworkError> {
        let links = self.list().await?;
        for link in links {
            if link.sender == sender && link.receiver == receiver {
                return Ok(Some(link.id));
            }
        }
        Ok(None)
    }

    /// Sends a custom action to the actor.
    pub async fn action(
        &self,
        id: Option<link_api::LinkId>,
        action: link_api::LinkAction,
    ) -> Result<(), actor_framework::FrameworkError> {
        self.inner.perform_action(id, action).await
    }

    /// Notifies the link actor that a chip has been added.
    pub async fn notify_chip_added(
        &self,
        chip_id: ChipId,
        kind: ChipKind,
    ) -> Result<(), actor_framework::FrameworkError> {
        self.action(None, link_api::LinkAction::NotifyChipAdded(chip_id, kind)).await
    }

    /// Notifies the link actor that a chip has been removed.
    pub async fn notify_chip_removed(
        &self,
        chip_id: ChipId,
    ) -> Result<(), actor_framework::FrameworkError> {
        self.action(None, link_api::LinkAction::NotifyChipRemoved(chip_id)).await
    }
}

#[async_trait::async_trait]
impl link_api::LinkClient for LinkClient {
    async fn list(&self) -> Result<Vec<link_api::Link>, String> {
        self.inner.list().await.map_err(|e| e.to_string())
    }

    async fn create(&self, params: link_api::LinkCreate) -> Result<link_api::LinkId, String> {
        self.inner.create(params).await.map_err(|e| e.to_string())
    }

    async fn update(
        &self,
        id: link_api::LinkId,
        patch: link_api::LinkUpdate,
    ) -> Result<(), String> {
        self.inner.update(id, patch).await.map(|_| ()).map_err(|e| e.to_string())
    }

    async fn delete(&self, id: link_api::LinkId) -> Result<(), String> {
        self.inner.delete(id).await.map_err(|e| e.to_string())
    }

    async fn action(
        &self,
        id: Option<link_api::LinkId>,
        action: link_api::LinkAction,
    ) -> Result<(), String> {
        self.inner.perform_action(id, action).await.map(|_| ()).map_err(|e| e.to_string())
    }

    async fn notify_chip_added(&self, chip_id: ChipId, kind: ChipKind) -> Result<(), String> {
        self.inner
            .perform_action(None, link_api::LinkAction::NotifyChipAdded(chip_id, kind))
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    async fn notify_chip_removed(&self, chip_id: ChipId) -> Result<(), String> {
        self.inner
            .perform_action(None, link_api::LinkAction::NotifyChipRemoved(chip_id))
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

impl Deref for LinkClient {
    type Target = ResourceClient<LinkActor>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
