//! # ActorService Trait
//!
//! The `ActorService` trait defines the contract that every resource (User, Product, Order, …) must implement to be managed by the generic `ResourceActor`. It specifies associated types for IDs, DTOs, actions, context, and errors, and provides lifecycle hooks (`on_create`, `on_update`, `on_delete`, `handle_action`). Implementing this trait enables the framework to offer a uniform CRUD + Action API for any domain model.
//!
//! Trait that any resource must implement to be managed by ResourceActor.
//!
//! # Architecture Note
//! Why do we need this trait?
//! By defining a contract (`ActorService`) that all our resource types (User, Product, Order)
//! must satisfy, we can write the `ResourceActor` logic *once* and reuse it everywhere.
//! This is "Polymorphism" in action.
/// Defines the business logic and behavior of an actor.
///
/// This trait corresponds to the **Business Logic** layer ("What I do").
/// It receives events/requests from the framework and mutates the actor's state.
///
/// We use "Associated Types" (type Id, type Create, etc.) to enforce type safety.
/// A `User` entity requires a `UserCreate` payload, and you can't accidentally send it
/// a `ProductCreate` payload. The compiler prevents this class of bugs entirely.
///
/// # Provided Methods (Hooks)
/// This trait includes **Provided Methods** (methods with default implementations) for lifecycle hooks:
/// - [`ActorService::on_create`]
/// - [`ActorService::on_delete`]
///
/// You do **not** need to implement these methods unless you want to customize behavior.
/// The default implementation does nothing (`Ok(())`).
use crate::lifecycle::ActorLifecycle;
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
/// It also defines a `Context` type, which is injected into every hook. This allows "Late Binding"
/// of dependencies (passing clients to `run()` instead of `new()`).
#[async_trait]
pub trait ActorService: Clone + Send + Sync + 'static {
    /// The unique identifier for this resource (e.g., String, Uuid, u64).
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
    type Context: ActorLifecycle + Send + Sync;

    /// The error type for this resource.
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

    /// Construct the full Resource from the ID and Payload.
    /// This is called synchronously before `on_create`.
    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error>;

    // --- Lifecycle Hooks (Async) ---

    /// Called immediately after the resource is created and initialized.
    /// Use this hook to perform validation or side effects (e.g., checking other actors).
    /// Called immediately after the resource is created and initialized.
    /// Use this hook to perform validation or side effects (e.g., checking other actors).
    async fn on_create(
        &mut self,
        actor: &mut Self::Context,
        framework: &mut impl Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called when an update request is received.
    async fn on_update(
        &mut self,
        _update: Self::Update,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called immediately before the resource is removed from the system.
    async fn on_delete(
        &self,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    // --- Action Handler (Async) ---

    /// Called when a custom action is received.
    async fn handle_action(
        &mut self,
        _action: Self::Action,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<Self::ActionResult, Self::Error>;

    /// Called when a list request is received.
    /// This method is static because it operates on the collection of resources.
    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Self::ListResponse;
}
