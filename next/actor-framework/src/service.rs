//! # ActorService Trait
//!
//! The `ActorService` trait defines the contract that every resource (User, Product, Order, …) must implement to be managed by the generic `ResourceActor`. It specifies associated types for IDs, DTOs, actions, context, and errors, and provides lifecycle hooks (`handle_create`, `handle_update`, `handle_delete`, `handle_action`). Implementing this trait enables the framework to offer a uniform CRUD + Action API for any domain model.

use crate::Context;
use async_trait::async_trait;
use bytes::Bytes;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::pin::Pin;
use tokio_stream::Stream;

pub type StreamMessage = Bytes;
pub type BoxStream = Pin<Box<dyn Stream<Item = StreamMessage> + Send>>;

/// Trait that any resource must implement to be managed by ResourceActor.
///
/// # Architecture Note
/// By defining a contract (`ActorService`) that all our resource types (User, Product, Order)
/// must satisfy, we can write the `ResourceActor` logic *once* and reuse it everywhere.
///
/// # Async & Context
/// This trait is `#[async_trait]` to allow asynchronous operations in hooks (e.g., calling other actors).
#[async_trait]
pub trait ActorService: Send + Sync + 'static {
    /// The unique identifier for this entity (e.g., String, Uuid, u64).
    /// Must be convertible from u32 for automatic ID generation.
    type Id: Eq + Hash + Clone + Send + Sync + Display + Debug + From<u32>;

    /// The data required to create a new instance (DTO - Data Transfer Object).
    type Create: Send + Sync + Debug;

    /// The data required to update an existing instance.
    type Update: Send + Sync + Debug;

    /// Enum representing resource-specific operations (e.g., `ReserveStock`).
    type Action: Send + Sync + Debug;

    /// The result type returned by custom actions.
    type ActionResult: Send + Sync + Debug;

    /// The error type for this entity.
    /// Must implement std::error::Error for proper error propagation.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The entity type returned by get/update/list operations.
    type Entity: Send + Sync + Debug + Clone;

    // --- Lifecycle Hooks (Async) ---

    /// Called when a create request is received.
    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        ctx: &mut impl Context,
    ) -> Result<Self::Id, Self::Error>;

    /// Called when a get request is received.
    async fn handle_get(
        &self,
        id: Self::Id,
        ctx: &mut impl Context,
    ) -> Result<Option<Self::Entity>, Self::Error>;

    /// Called when an update request is received.
    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        ctx: &mut impl Context,
    ) -> Result<Self::Entity, Self::Error>;

    /// Called when a delete request is received for a specific entity.
    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error>;

    // --- Action Handler (Async) ---

    /// Handles a custom action.
    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut impl Context,
    ) -> Result<Self::ActionResult, Self::Error> {
        unimplemented!("handle_action not implemented")
    }

    /// Called when a list request is received.
    async fn handle_list(
        &mut self,
        ctx: &mut impl Context,
    ) -> Result<Vec<Self::Entity>, Self::Error>;
}
