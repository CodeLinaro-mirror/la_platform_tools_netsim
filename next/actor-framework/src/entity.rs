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

use async_trait::async_trait;
use std::fmt::{Debug, Display};
use std::hash::Hash;

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
    type Context: Send + Sync;

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

    /// Construct the full Entity from the ID and Payload.
    /// This is called synchronously before `on_create`.
    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error>;

    // --- Lifecycle Hooks (Async) ---

    /// Called immediately after the entity is created and initialized.
    /// Use this hook to perform validation or side effects (e.g., checking other actors).
    async fn on_create(&mut self, _ctx: &Self::Context) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called when an update request is received.
    async fn on_update(
        &mut self,
        update: Self::Update,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error>;

    /// Called immediately before the entity is removed from the system.
    async fn on_delete(&self, _ctx: &Self::Context) -> Result<(), Self::Error> {
        Ok(())
    }

    // --- Action Handler (Async) ---

    /// Handle a custom resource-specific action.
    async fn handle_action(
        &mut self,
        action: Self::Action,
        _ctx: &Self::Context,
    ) -> Result<Self::ActionResult, Self::Error>;
}
