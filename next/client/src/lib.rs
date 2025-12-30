// Copyright (C) 2025 The Android Open Source Project

//! # Domain-Specific Client Wrappers
//!
//! This module provides **type-safe, domain-specific wrappers** around the generic
//! [`ResourceClient`](actor_framework::ResourceClient). These wrappers add domain
//! knowledge and ergonomic APIs on top of the raw framework client.
//!
//! ## The Client Wrapper Pattern
//!
//! Instead of exposing the generic `ResourceClient<T>` directly, we wrap it in
//! domain-specific clients that provide:
//!
//! 1. **Domain-specific methods** - `create_user()` instead of generic `create()`
//! 2. **Type-safe errors** - `UserError` instead of generic `FrameworkError`
//! 3. **Business logic** - Validation, transformation, orchestration
//! 4. **Better API ergonomics** - Hide framework details from consumers
//!
//! ## Example: UserClient
//!
//! ```rust
//! use actor_framework::ResourceClient;
//! use actor_sample::model::{User, UserCreate, UserId};
//! use actor_sample::user_actor::UserError;
//!
//! #[derive(Clone)]
//! pub struct UserClient {
//!     inner: ResourceClient<User>,
//! }
//!
//! impl UserClient {
//!     pub fn new(inner: ResourceClient<User>) -> Self {
//!         Self { inner }
//!     }
//!
//!     // Domain-specific method with type-safe errors
//!     pub async fn create_user(&self, params: UserCreate) -> Result<UserId, UserError> {
//!         self.inner.create(params).await
//!             .map_err(|e| UserError::ActorCommunicationError(e.to_string()))
//!     }
//! }
//! ```
//!
//! ## The ActorClient Trait
//!
//! The [`crate::actor_client::ActorClient`] trait provides a common interface for all clients,
//! automatically implementing `get()` and `delete()` methods:
//!
//! ```rust,ignore
//! #[async_trait]
//! impl ActorClient<User> for UserClient {
//!     type Error = UserError;
//!
//!     fn inner(&self) -> &ResourceClient<User> {
//!         &self.inner
//!     }
//!
//!     fn map_error(e: FrameworkError) -> Self::Error {
//!         UserError::ActorCommunicationError(e.to_string())
//!     }
//! }
//! ```
//!
//! Now `UserClient` automatically gets:
//! - `async fn get(&self, id: String) -> Result<Option<User>, UserError>`
//! - `async fn delete(&self, id: String) -> Result<(), UserError>`
//!
//! ## Type-Safe Error Mapping
//!
//! Each client maps framework errors to domain-specific error types:
//!
//! ```rust,ignore
//! // Framework error (generic)
//! FrameworkError::Timeout
//!
//! // Mapped to domain error (specific)
//! UserError::ActorCommunicationError("timeout".to_string())
//! ```
//!
//! This allows consumers to pattern match on domain-specific errors:
//!
//! ```rust,ignore
//! match user_client.get(id).await {
//!     Ok(Some(user)) => println!("Found: {}", user.name),
//!     Ok(None) => println!("User not found"),
//!     Err(UserError::ActorCommunicationError(msg)) => {
//!         println!("Communication failed: {}", msg)
//!     }
//!     Err(e) => println!("Other error: {}", e),
//! }
//! ```
//!
//! ## Orchestration Example: OrderClient
//!
//! Clients can orchestrate multiple actors to implement complex workflows:
//!
//! ```rust,ignore
//! impl OrderClient {
//!     pub async fn create_order(&self, params: OrderCreate) -> Result<String, OrderError> {
//!         // 1. Validate user exists
//!         let user = self.user_client.get(params.user_id.clone()).await?
//!             .ok_or_else(|| OrderError::InvalidUser(params.user_id.clone()))?;
//!
//!         // 2. Reserve product stock
//!         self.product_client.reserve_stock(
//!             params.product_id.clone(),
//!             params.quantity
//!         ).await?;
//!
//!         // 3. Create the order
//!         match self.inner.create(params.clone()).await {
//!             Ok(id) => Ok(id),
//!             Err(e) => {
//!                 // COMPENSATING TRANSACTION: Rollback stock reservation
//!                 // If we fail to create the order, we must release the stock
//!                 // so it doesn't get "leaked" (permanently reserved).
//!                 let _ = self.product_client.release_stock(
//!                     params.product_id,
//!                     params.quantity
//!                 ).await;
//!
//!                 Err(OrderError::ActorCommunicationError(e.to_string()))
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! This keeps orchestration logic in the **client layer**, while actors remain
//! focused on managing their own state.
//!
//! ## Benefits
//!
//! 1. **Type Safety** - Compile-time guarantees for domain operations
//! 2. **Encapsulation** - Hide framework details from consumers
//! 3. **Testability** - Easy to mock with [`MockClient`](actor_framework::mock::MockClient)
//! 4. **Maintainability** - Domain logic lives in one place
//! 5. **Discoverability** - IDE autocomplete shows domain methods

pub mod device_client;
pub use device_client::*;
pub mod capture_client;
pub use capture_client::*;
pub use link_actor::LinkClient;
pub mod mock_chip_client;
pub use mock_chip_client::{MockChipClient, MockChipEntity};
