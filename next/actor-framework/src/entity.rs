//! # ActorEntity Trait
//!
//! The `ActorEntity` trait defines the contract that every resource (User, Product, Order, …) must implement to be managed by the generic `ResourceActor`. It specifies associated types for IDs, DTOs, actions, context, and errors, and provides lifecycle hooks (`on_create`, `on_update`, `on_delete`, `handle_action`). Implementing this trait enables the framework to offer a uniform CRUD + Action API for any domain model.
//!
//! Trait that any resource entity must implement to be managed by ResourceActor.
//!
//! # Architecture Note
//! Why do we need this trait?
//! By defining a contract (`ActorEntity`) that all our resource types (User, Product, Order)
//! must satisfy, we can write the `ResourceActor` logic *once* and reuse it everywhere.
//! This is "Polymorphism" in action.
//!
//! We use "Associated Types" (type Id, type Create, etc.) to enforce type safety.
//! A `User` entity requires a `UserCreate` payload, and you can't accidentally send it
//! a `ProductCreate` payload. The compiler prevents this class of bugs entirely.
//!
//! # Provided Methods (Hooks)
//! This trait includes **Provided Methods** (methods with default implementations) for lifecycle hooks:
//! - [`ActorEntity::on_create`]
//! - [`ActorEntity::on_delete`]
//!
//! You do **not** need to implement these methods unless you want to customize behavior.
//! The default implementation does nothing (`Ok(())`).

use crate::runtime::Runtime;
use async_trait::async_trait;
use bytes::Bytes;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::pin::Pin;
use tokio_stream::Stream;

pub type StreamMessage = Bytes;
pub type BoxStream = Pin<Box<dyn Stream<Item = StreamMessage> + Send>>;

/// The context trait that defines the lifecycle hooks for an actor entity.
///
/// This trait allows the entity to interact with the actor runtime (timers, streams, shutdown)
/// and handle asynchronous events.
#[async_trait]
pub trait ActorContext: Send + Sync + 'static {
    /// The error type returned by context methods.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Called when the actor starts, before processing any messages.
    ///
    /// Use this hook to:
    /// - Schedule initial timers.
    /// - Register initial streams.
    async fn on_start(&mut self, _runtime: &mut impl Runtime) {}

    /// Called on every tick of the actor's interval.
    async fn on_tick(&mut self, _runtime: &mut impl Runtime) {}

    /// Called when a stream produces a message.
    async fn on_stream(
        &mut self,
        _id: usize,
        _message: StreamMessage,
        _runtime: &mut impl Runtime,
    ) {
    }

    /// Called when a registered stream closes.
    ///
    /// Returns `Ok(true)` if the entity associated with the stream ID should be deleted.
    /// This is useful for entities that are 1:1 with a stream (e.g., a chip connected to a packet stream).
    async fn on_stream_closed(&mut self, _id: usize) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Called when the actor receives a shutdown signal.
    async fn on_shutdown(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct EmptyError;
impl std::fmt::Display for EmptyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmptyError")
    }
}
impl std::error::Error for EmptyError {}

#[async_trait]
impl ActorContext for () {
    type Error = EmptyError;
}

/// Trait that any resource entity must implement to be managed by ResourceActor.
///
/// # Architecture Note
/// By defining a contract (`ActorEntity`) that all our resource types (User, Product, Order)
/// must satisfy, we can write the `ResourceActor` logic *once* and reuse it everywhere.
///
/// # Async & Context
/// This trait is `#[async_trait]` to allow asynchronous operations in hooks (e.g., calling other actors).
/// It also defines a `Context` type, which is injected into every hook. This allows "Late Binding"
/// of dependencies (passing clients to `run()` instead of `new()`).
#[async_trait]
pub trait ActorEntity: Clone + Send + Sync + 'static {
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

    /// The runtime context (dependencies) injected into the actor.
    /// Use `()` if no dependencies are needed.
    type Context: ActorContext + Send + Sync;

    /// The error type for this entity.
    /// Must implement std::error::Error for proper error propagation.
    ///
    /// # Design Note: Error Granularity
    ///
    /// The framework enforces a **Per-Actor Error Type** (one enum for the whole actor) rather than
    /// **Per-Message Error Types** (a specific error for each action).
    ///
    /// **Why?**
    /// - **Simplicity**: Reduces boilerplate. You don't need to define 10 different error enums for 10 actions.
    /// - **Ergonomics**: Clients deal with a single `UserError` type, making pattern matching easier.
    ///
    /// **Trade-off**:
    /// This means `UserError` must be the union of all possible errors. If `ActionA` can only fail with `ErrorX`,
    /// but `ActionB` can fail with `ErrorY`, the return type for both is `Result<..., UserError>`, which technically
    /// allows `ErrorY` to be returned from `ActionA`. In practice, this theoretical loss of precision is worth
    /// the massive reduction in code complexity.
    type Error: std::error::Error + Send + Sync + 'static;
    type ListResponse: Send + Sync + Debug;

    /// Construct the full Entity from the ID and Payload.
    /// This is called synchronously before `on_create`.
    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error>;

    // --- Lifecycle Hooks (Async) ---

    /// Called immediately after the entity is created and initialized.
    /// Use this hook to perform validation or side effects (e.g., checking other actors).
    /// Called immediately after the entity is created and initialized.
    /// Use this hook to perform validation or side effects (e.g., checking other actors).
    async fn on_create(
        &mut self,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called when an update request is received.
    async fn on_update(
        &mut self,
        _update: Self::Update,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called immediately before the entity is removed from the system.
    async fn on_delete(
        &self,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    // --- Action Handler (Async) ---

    /// Called when a custom action is received.
    async fn handle_action(
        &mut self,
        _action: Self::Action,
        _context: &mut Self::Context,
        _runtime: &mut impl Runtime,
    ) -> Result<Self::ActionResult, Self::Error>;

    /// Called when a list request is received.
    /// This method is static because it operates on the collection of entities.
    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        context: &mut Self::Context,
        runtime: &mut impl Runtime,
    ) -> Self::ListResponse;
}
