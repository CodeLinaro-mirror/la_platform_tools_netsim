// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! # ActorService Trait
//!
//! The `ActorService` trait defines the contract that every resource (User,
//! Product, Order, …) must implement to be managed by the generic
//! `ResourceActor`. It specifies associated types for IDs, DTOs, actions,
//! context, and errors, and provides lifecycle hooks (`handle_create`,
//! `handle_update`, `handle_delete`, `handle_action`). Implementing this trait
//! enables the framework to offer a uniform CRUD + Action API for any domain
//! model.

use std::{
    fmt::{Debug, Display},
    hash::Hash,
    pin::Pin,
};

use bytes::Bytes;
use tokio_stream::Stream;

use crate::DynContext;

pub type StreamMessage = Bytes;
pub type BoxStream = Pin<Box<dyn Stream<Item = StreamMessage> + Send>>;
pub type BoxTypedStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

/// Trait alias for Actor IDs ensuring all required bounds are met.
pub trait ActorId:
    Eq + Hash + Clone + Send + Sync + Copy + Display + Debug + From<u32> + Into<u32> + Unpin + 'static
{
}
impl<T> ActorId for T where
    T: Eq
        + Hash
        + Clone
        + Send
        + Sync
        + Copy
        + Display
        + Debug
        + From<u32>
        + Into<u32>
        + Unpin
        + 'static
{
}

/// Trait that any resource must implement to be managed by ResourceActor.
///
/// # Architecture Note
/// By defining a contract (`ActorService`) that all our resource types (User,
/// Product, Order) must satisfy, we can write the `ResourceActor` logic *once*
/// and reuse it everywhere.
pub trait ActorService: Send + Sync + 'static {
    /// The unique identifier for this entity (e.g., String, Uuid, u64).
    /// Must be convertible from u32 for automatic ID generation.
    type Id: ActorId;

    /// The data required to create a new instance (DTO - Data Transfer Object).
    type Create: Send + Sync + Debug;

    /// The data required to update an existing instance.
    type Update: Send + Sync + Debug;

    /// Enum representing resource-specific operations (e.g., `ReserveStock`).
    type Action: Send + Sync + Debug;

    /// The result type returned by custom actions.
    type ActionResult: Send + Debug;

    /// The error type for this entity.
    /// Must implement std::error::Error for proper error propagation.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The entity type returned by get/update/list operations.
    type Entity: Send + Sync + Debug + Clone;

    /// The item type for the alternative typed stream map.
    type TypedStream: Send + Sync + Debug + Clone + Unpin + 'static;

    // --- Lifecycle Hooks (Async) ---
    //
    /// These hooks are called sequentially in the actor's run loop.
    /// Blocking logic or long-running CPU tasks here will block the entire
    /// actor. Use `ctx.spawn()` for heavy tasks.
    ///
    /// Called when a create request is received.
    fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = Result<Self::Id, Self::Error>> + Send;

    /// Called when a get request is received.
    fn handle_get(
        &self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = Result<Option<Self::Entity>, Self::Error>> + Send;

    /// Called when an update request is received.
    fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = Result<Self::Entity, Self::Error>> + Send;

    /// Called when a delete request is received for a specific entity.
    fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;

    // --- Action Handler (Async) ---

    /// Handles a custom action.
    fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = Result<Self::ActionResult, Self::Error>> + Send;

    /// Called when a list request is received.
    fn handle_list(
        &mut self,
        ctx: &mut DynContext<Self>,
    ) -> impl std::future::Future<Output = Result<Vec<Self::Entity>, Self::Error>> + Send;
}
