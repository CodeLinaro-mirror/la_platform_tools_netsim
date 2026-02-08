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

use async_trait::async_trait;
use bytes::Bytes;
use tokio_stream::Stream;

use crate::DynContext;

pub type StreamMessage = Bytes;
pub type BoxStream = Pin<Box<dyn Stream<Item = StreamMessage> + Send>>;

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
///
/// # Async & Context
/// This trait is `#[async_trait]` to allow asynchronous operations in hooks
/// (e.g., calling other actors).
#[async_trait]
pub trait ActorService: Send + Sync + 'static {
    /// The unique identifier for this entity (e.g., String, Uuid, u64).
    /// Must be convertible from u32 for automatic ID generation.
    type Id: ActorId;

    /// The data required to create a new instance (DTO - Data Transfer Object).
    type Create: Send + Sync + Debug;

    /// The data required to update an existing instance.
    type Update: Send + Sync + Debug;

    /// Enum representing resource-specific operations (e.g., `ReserveStock`).
    type Action: Send + Debug;

    /// The result type returned by custom actions.
    type ActionResult: Send + Debug;

    /// The error type for this entity.
    /// Must implement std::error::Error for proper error propagation.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The entity type returned by get/update/list operations.
    type Entity: Send + Sync + Debug + Clone;

    // --- Lifecycle Hooks (Async) ---
    //
    /// These hooks are called sequentially in the actor's run loop.
    /// Blocking logic or long-running CPU tasks here will block the entire
    /// actor. Use `ctx.spawn()` for heavy tasks.

    /// Called when a create request is received.
    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error>;

    /// Called when a get request is received.
    async fn handle_get(
        &self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error>;

    /// Called when an update request is received.
    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error>;

    /// Called when a delete request is received for a specific entity.
    async fn handle_delete(
        &mut self,
        id: Self::Id,
        ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error>;

    // --- Action Handler (Async) ---

    /// Handles a custom action.
    /// Handles a custom action.
    async fn handle_action(
        &mut self,
        id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error>;

    /// Called when a list request is received.
    async fn handle_list(
        &mut self,
        ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error>;
}
