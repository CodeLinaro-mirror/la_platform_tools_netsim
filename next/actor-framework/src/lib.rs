//! # Actor Framework
//!
//! This crate provides the foundational building blocks for creating type-safe, concurrent
//! actor systems in Rust. It implements a **Resource-Oriented Architecture (ROA)** pattern
//! on top of the **Actor Model**, providing a clean abstraction for managing stateful entities.
//!
//! ## Why ROA + Actor Model?
//!
//! This framework combines **Resource-Oriented Architecture (ROA)** with the **Actor Model**
//! to create a powerful pattern for building scalable systems.
//!
//! ### Resource-Oriented Architecture (ROA)
//!
//! - Standard CRUD operations (Create, Read, Update, Delete) on well-defined resources
//! - Predictable lifecycle management
//! - Clean, uniform API surface across all resource types
//!
//! ### Actor Model
//!
//! - Isolated state (no shared memory, no locks)
//! - Message-passing concurrency
//! - Sequential processing within each actor eliminates race conditions
//!
//! ### The Synergy
//!
//! - **Separation**: Each resource type (User, Product, Order) gets its own actor with completely isolated state
//! - **Coordination**: When resources need to interact (e.g., Order reserving Product stock), they communicate via **Action messages** instead of direct coupling
//! - **Scalability**: Independent resources can scale independently without coordination overhead
//! - **Maintainability**: Changes to one resource type don't ripple through the system
//!
//! This pattern excels in systems with many loosely-coupled resources that occasionally need
//! to coordinate. The ROA provides structure, while the Actor Model provides safe concurrency.
//!
//! **Further Reading**:
//! - [Actor Model (Wikipedia)](https://en.wikipedia.org/wiki/Actor_model) - Foundational concurrency pattern by Carl Hewitt
//! - [Resource-Oriented Architecture](https://www.ics.uci.edu/~fielding/pubs/dissertation/rest_arch_style.htm) - Roy Fielding's dissertation on REST/ROA principles
//! - [Actors in Rust](https://ryhl.io/blog/actors-with-tokio/) - Practical guide to implementing actors with Tokio
//!
//! ## Architecture Overview
//!
//! The framework separates concerns into three layers:
//!
//! 1. **Entity Layer** ([`ActorEntity`]) - Your business logic and domain models
//! 2. **Runtime Layer** ([`ResourceActor`]) - Message processing and concurrency
//! 3. **Interface Layer** ([`ResourceClient`]) - Type-safe communication
//!
//! This separation means you write your business logic **once** in the entity trait,
//! and the framework handles all the async message passing, error handling, and state management.
//!
//! ## Core Abstractions
//!
//! ### [`ActorEntity`] - The Business Logic
//!
//! Define what your actor manages and how it behaves:
//!
//! ### [`ActorEntity`] - The Business Logic
//!
//! Define what your actor manages and how it behaves:
//!
//! ```rust
//! use actor_framework::{ActorEntity, ResourceActor, ResourceClient};
//! use async_trait::async_trait;
//!
//! // 1. Define the Entity
//! #[derive(Clone, Debug)]
//! struct User {
//!     id: u32,
//!     name: String,
//! }
//!
//! #[derive(Debug)] struct UserCreate { name: String }
//! #[derive(Debug)] struct UserUpdate { name: Option<String> }
//! #[derive(Debug)] enum UserAction {}
//! #[derive(Debug)] struct UserError(String);
//!
//! impl std::fmt::Display for UserError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
//! }
//! impl std::error::Error for UserError {}
//!
//! #[async_trait]
//! impl ActorEntity for User {
//!     type Id = u32;
//!     type Create = UserCreate;
//!     type Update = UserUpdate;
//!     type Action = UserAction;
//!     type ActionResult = ();
//!     type Context = ();
//!     type Error = UserError;
//!
//!     fn from_create_params(id: u32, params: UserCreate) -> Result<Self, Self::Error> {
//!         Ok(Self { id, name: params.name })
//!     }
//!
//!     async fn on_update(&mut self, update: UserUpdate, _ctx: &Self::Context) -> Result<(), Self::Error> {
//!         if let Some(name) = update.name { self.name = name; }
//!         Ok(())
//!     }
//!
//!     async fn handle_action(&mut self, _: UserAction, _: &Self::Context) -> Result<(), Self::Error> {
//!         Ok(())
//!     }
//! }
//!
//! // 2. Use the Actor
//! #[tokio::main]
//! async fn main() {
//!     // Create actor and client
//!     let (actor, client) = ResourceActor::<User>::new(10);
//!
//!     // Spawn the actor
//!     tokio::spawn(actor.run(()));
//!
//!     // Use the client
//!     let id = client.create(UserCreate { name: "Alice".into() }).await.unwrap();
//!     let user = client.get(id).await.unwrap().unwrap();
//!     assert_eq!(user.name, "Alice");
//! }
//! ```
//!
//! ## Context Injection Pattern
//!
//! Dependencies are injected at **runtime** via the `run()` method, not at construction time.
//! This "late binding" pattern solves circular dependencies:
//!
//! ```rust
//! use actor_framework::{ActorEntity, ResourceActor, ResourceClient};
//! use async_trait::async_trait;
//!
//! // --- Define Minimal Entities ---
//! #[derive(Clone, Debug)] struct User { id: u32 }
//! #[derive(Debug)] struct UserCreate;
//! #[derive(Debug)] struct UserUpdate;
//! #[derive(Debug)] enum UserAction {}
//! #[derive(Debug)] struct UserError;
//! impl std::fmt::Display for UserError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Err") } }
//! impl std::error::Error for UserError {}
//! impl From<String> for UserError { fn from(_: String) -> Self { UserError } }
//!
//! #[async_trait]
//! impl ActorEntity for User {
//!     type Id = u32; type Create = UserCreate; type Update = UserUpdate; type Action = UserAction;
//!     type ActionResult = (); type Context = (); type Error = UserError;
//!     fn from_create_params(id: u32, _: UserCreate) -> Result<Self, Self::Error> { Ok(Self { id }) }
//!     async fn on_update(&mut self, _: UserUpdate, _: &()) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_action(&mut self, _: UserAction, _: &()) -> Result<(), Self::Error> { Ok(()) }
//! }
//!
//! #[derive(Clone, Debug)] struct Product { id: u32 }
//! // ... (impl ActorEntity for Product similar to User) ...
//! # #[derive(Debug)] struct ProductCreate;
//! # #[derive(Debug)] struct ProductUpdate;
//! # #[derive(Debug)] enum ProductAction {}
//! # #[derive(Debug)] struct ProductError;
//! # impl std::fmt::Display for ProductError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Err") } }
//! # impl std::error::Error for ProductError {}
//! # impl From<String> for ProductError { fn from(_: String) -> Self { ProductError } }
//! # #[async_trait]
//! # impl ActorEntity for Product {
//! #     type Id = u32; type Create = ProductCreate; type Update = ProductUpdate; type Action = ProductAction;
//! #     type ActionResult = (); type Context = (); type Error = ProductError;
//! #     fn from_create_params(id: u32, _: ProductCreate) -> Result<Self, Self::Error> { Ok(Self { id }) }
//! #     async fn on_update(&mut self, _: ProductUpdate, _: &()) -> Result<(), Self::Error> { Ok(()) }
//! #     async fn handle_action(&mut self, _: ProductAction, _: &()) -> Result<(), Self::Error> { Ok(()) }
//! # }
//!
//! #[derive(Clone, Debug)] struct Order { id: u32 }
//! // Order depends on UserClient and ProductClient
//! type OrderContext = (ResourceClient<User>, ResourceClient<Product>);
//!
//! #[derive(Debug)] struct OrderCreate;
//! #[derive(Debug)] struct OrderUpdate;
//! #[derive(Debug)] enum OrderAction {}
//! #[derive(Debug)] struct OrderError;
//! impl std::fmt::Display for OrderError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Err") } }
//! impl std::error::Error for OrderError {}
//! impl From<String> for OrderError { fn from(_: String) -> Self { OrderError } }
//!
//! #[async_trait]
//! impl ActorEntity for Order {
//!     type Id = u32; type Create = OrderCreate; type Update = OrderUpdate; type Action = OrderAction;
//!     type ActionResult = (); type Context = OrderContext; type Error = OrderError;
//!
//!     fn from_create_params(id: u32, _: OrderCreate) -> Result<Self, Self::Error> { Ok(Self { id }) }
//!     async fn on_update(&mut self, _: OrderUpdate, _: &OrderContext) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_action(&mut self, _: OrderAction, _: &OrderContext) -> Result<(), Self::Error> { Ok(()) }
//!     // In a real app, on_create would use the context to validate user/product
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // 1. Create all actors (no dependencies yet)
//!     let (user_actor, user_client) = ResourceActor::<User>::new(10);
//!     let (product_actor, product_client) = ResourceActor::<Product>::new(10);
//!     let (order_actor, order_client) = ResourceActor::<Order>::new(10);
//!
//!     // 2. Wire dependencies when starting actors
//!     tokio::spawn(user_actor.run(()));
//!     tokio::spawn(product_actor.run(()));
//!     // Order actor gets the clients it needs
//!     tokio::spawn(order_actor.run((user_client, product_client)));
//!
//!     // 3. Use the actor (keeps main alive)
//!     let _ = order_client.create(OrderCreate).await;
//! }
//! ```
//!
//! The `Order` actor receives `(UserClient, ProductClient)` as its context, allowing it to
//! validate users and reserve product stock during order creation.
//!
//! ## Type Safety
//!
//! The framework leverages Rust's type system to eliminate entire classes of runtime errors:
//!
//! - **Compile-time guarantees**: Can't send wrong message types to actors
//! - **Type-safe errors**: Each entity defines its own error type
//! - **No stringly-typed APIs**: IDs, actions, and results are all strongly typed
//!
//! ## Concurrency Model
//!
//! - Each actor runs in its own Tokio task
//! - Messages are processed **sequentially** within an actor (no locks needed!)
//! - Multiple actors run in **parallel** (true concurrency)
//! - No shared mutable state (message passing only)
//!
//! ## Testing
//!
//! The framework provides a **MockClient** type that implements the same `ResourceClient<T>` API as the real client but operates entirely in‑memory. It lets you write fast, deterministic unit tests for client logic (e.g. `OrderClient`) without spawning any actors. See the [`mock`] module for the full API and usage patterns.

pub mod actor;
pub mod client;
pub mod client_trait;
pub mod entity;
pub mod error;
pub mod message;
pub mod mock;
// pub mod tracing;

// Re-export core types for convenience
pub use actor::ResourceActor;
pub use client::ResourceClient;
pub use client_trait::ActorClient;
pub use entity::ActorEntity;
pub use error::FrameworkError;
pub use message::{ResourceRequest, Response};
