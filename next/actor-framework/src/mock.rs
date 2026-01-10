// Copyright 2025 The Android Open Source Project

//! # Mock Framework & Testing Guide
//!
//! The `MockClient<T>` type implements the same `ResourceClient<T>` API as the production client but operates entirely in‑memory. It lets you set expectations and return values for unit tests, enabling fast, deterministic testing of client logic without spawning any actors.
//!
//! ## When to use Mocks vs Real Actors
//!
//! | Feature | MockClient | Real Actor |
//! |---------|------------|------------|
//! | **Speed** | Instant (in-memory) | Fast (but involves tokio spawn) |
//! | **Determinism** | 100% Deterministic | Subject to scheduler |
//! | **State** | No real state (expectations) | Real state management |
//! | **Use Case** | Unit testing logic *around* the client | Testing the actor itself or full system |
//! | **Error Injection** | Easy (`return_err`) | Hard (requires specific state) |
//!
//! ## Testing Strategies
//!
//! The actor framework supports four distinct testing patterns.
//!
//! <details>
//! <summary><b>Pattern 0: Client Logic Test (Pure Mock)</b></summary>
//!
//! **When to use**: Testing complex orchestration logic in your client wrappers without spinning up any actors.
//!
//! **Example**:
//! ```rust
//! use actor_framework::mock::MockClient;
//! use actor_framework::{ActorService, Context, DynContext, ResourceClient, ResourceRequest};
//! use async_trait::async_trait;
//!
//! // --- Define a minimal Service for the test ---
//! #[derive(Clone, Debug, PartialEq)]
//! struct User { id: u32, email: String }
//! #[derive(Debug)] struct UserCreate { email: String }
//! #[derive(Debug)] struct UserUpdate;
//! #[derive(Debug)] enum UserAction {}
//! #[derive(Debug, thiserror::Error)] #[error("User error")] struct UserError;
//!
//! #[async_trait]
//! impl ActorService for User {
//!     type Id = u32; type Create = UserCreate; type Update = UserUpdate;
//!     type Action = UserAction; type ActionResult = (); type Error = UserError; type Entity = User;
//!
//!     async fn handle_create(
//!         &mut self,
//!         id: Option<u32>,
//!         params: UserCreate,
//!         _: &mut DynContext<Self::Id>,
//!     ) -> Result<u32, Self::Error> {
//!         let id = id.unwrap_or(0);
//!         self.id = id;
//!         self.email = params.email;
//!         Ok(id)
//!     }
//!     async fn handle_get(&self, _: u32, _: &mut DynContext<Self::Id>) -> Result<Option<Self::Entity>, Self::Error> { Ok(Some(self.clone())) }
//!     async fn handle_update(&mut self, _: u32, _: UserUpdate, _: &mut DynContext<Self::Id>) -> Result<Self::Entity, Self::Error> { Ok(self.clone()) }
//!     async fn handle_delete(&mut self, _: u32, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_action(&mut self, _: Option<u32>, _: UserAction, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_list(&mut self, _: &mut DynContext<Self::Id>) -> Result<Vec<User>, Self::Error> { Ok(vec![self.clone()]) }
//! }
//!
//! // --- Define a minimal Client Wrapper ---
//! struct UserClient { client: ResourceClient<User> }
//! impl UserClient {
//!     fn new(client: ResourceClient<User>) -> Self { Self { client } }
//!     async fn get(&self, id: u32) -> Result<Option<User>, UserError> {
//!         self.client.get(id).await.map_err(|_| UserError)
//!     }
//! }
//!
//! impl User {
//!     fn new(id: u32, email: &str) -> Self { Self { id, email: email.to_string() } }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // 1. Setup Mocks
//!     let mut user_mock = MockClient::<User>::new();
//!     user_mock.expect_get(1)
//!         .return_ok(Some(User::new(1, "test@example.com")));
//!
//!     // 2. Create Client with Mocks
//!     let user_client = UserClient::new(user_mock.client());
//!
//!     // 3. Test Logic
//!     let user = user_client.get(1).await.unwrap();
//!     assert_eq!(user.unwrap().email, "test@example.com");
//! }
//! ```
//! </details>
//!
//! <details>
//! <summary><b>Pattern 1: Single Actor Test (Fast, Isolated)</b></summary>
//!
//! **When to use**: Testing a single actor's logic in isolation.
//!
//! **Example**:
//! ```rust
//! use actor_framework::{ActorId, ActorLifecycle, ActorService, BoxStream, Context, DynContext, ResourceActor, ResourceClient};
//! use async_trait::async_trait;
//!
//! // --- Define Service ---
//! #[derive(Clone, Debug)] struct Product { id: u32, stock: u32 }
//! #[derive(Debug)] struct ProductCreate { stock: u32 }
//! #[derive(Debug)] struct ProductUpdate;
//! #[derive(Debug)] enum ProductAction { CheckStock }
//! #[derive(Debug, thiserror::Error)] #[error("Err")] struct ProductError;
//!
//! #[async_trait]
//! impl ActorService for Product {
//!     type Id = u32; type Create = ProductCreate; type Update = ProductUpdate;
//!     type Action = ProductAction; type ActionResult = u32; type Error = ProductError; type Entity = Product;
//!
//!     async fn handle_create(&mut self, id: Option<u32>, params: ProductCreate, _: &mut DynContext<Self::Id>) -> Result<u32, Self::Error> {
//!         let id = id.unwrap_or(0);
//!         self.id = id;
//!         self.stock = params.stock;
//!         Ok(id)
//!     }
//!     async fn handle_get(&self, _: u32, _: &mut DynContext<Self::Id>) -> Result<Option<Self::Entity>, Self::Error> { Ok(Some(self.clone())) }
//!     async fn handle_update(&mut self, _: u32, _: ProductUpdate, _: &mut DynContext<Self::Id>) -> Result<Self::Entity, Self::Error> { Ok(self.clone()) }
//!     async fn handle_delete(&mut self, _: u32, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_action(&mut self, _: Option<u32>, action: ProductAction, _: &mut DynContext<Self::Id>) -> Result<u32, Self::Error> {
//!         match action { ProductAction::CheckStock => Ok(self.stock) }
//!     }
//!     async fn handle_list(&mut self, _: &mut DynContext<Self::Id>) -> Result<Vec<Product>, Self::Error> { Ok(vec![self.clone()]) }
//! }
//!
//! #[async_trait]
//! impl ActorLifecycle<u32> for Product {
//!     type Error = ProductError;
//!     async fn on_start(&mut self, _ctx: &mut DynContext<u32>) {}
//!     async fn on_tick(&mut self, _ctx: &mut DynContext<u32>) {}
//!     async fn on_stream(&mut self, _id: u32, _msg: bytes::Bytes, _ctx: &mut DynContext<u32>) {}
//!     async fn on_stream_closed(&mut self, _id: u32, _ctx: &mut DynContext<u32>) {}
//!     async fn on_task_closed(&mut self, _id: u32, _ctx: &mut DynContext<u32>) {}
//! }
//!
//! // --- MockContext for testing ---
//! struct MockContext<Id>(std::marker::PhantomData<Id>);
//!
//! impl<Id> Context<Id> for MockContext<Id>
//! where
//!     Id: Send + 'static,
//! {
//!     fn set_interval(&mut self, _duration: std::time::Duration) {}
//!
//!     fn add_stream(&mut self, _id: Id, _stream: BoxStream) {}
//!
//!     fn remove_stream(&mut self, _id: Id) {}
//!
//!     fn spawn(&mut self, _id: Id, _task: std::pin::Pin<Box<dyn std::future::Future<Output = Id> + Send>>) {}
//!
//!     fn abort(&mut self, _id: Id) {}
//!
//!     fn shutdown(&mut self) {}
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let product = Product { id: 0, stock: 0 };
//!     let (actor, client) = ResourceActor::new(10);
//!     tokio::spawn(actor.run(product));
//!
//!     let params = ProductCreate { stock: 100 };
//!     let id = client.create(params).await.unwrap();
//!     let stock = client.perform_action(Some(id), ProductAction::CheckStock).await.unwrap();
//!     assert_eq!(stock, 100);
//! }
//! ```
//! </details>
//!
//! <details>
//! <summary><b>Pattern 2: Actor with Mocked Dependencies (Sweet Spot)</b></summary>
//!
//! **When to use**: Testing an actor that depends on other actors, but you want to isolate the actor under test.
//!
//! **Example**:
//! ```text
//! This example requires multiple actors and is verbose to implement inline.
//! See tests/order_actor_test.rs in the actor-recipe-app crate for a full example.
//! ```
//! </details>
//!
//! <details>
//! <summary><b>Pattern 3: Full System Integration Test (Comprehensive)</b></summary>
//!
//! **When to use**: Testing the entire system working together, end-to-end flows, concurrency.
//!
//! See the `test_full_order_system_integration` function in `tests/integration_test.rs` for comprehensive examples.
//! </details>
//!
//! ## Testing Failure Scenarios
//!
//! One of the biggest advantages of `MockClient` is the ability to simulate errors that are hard to reproduce with real actors (e.g., database timeouts, network partitions).
//!
//! ```rust
//! use actor_framework::mock::MockClient;
//! use actor_framework::{ActorService, Context, DynContext, FrameworkError};
//! use async_trait::async_trait;
//!
//! #[derive(Clone, Debug)] struct User { id: u32 }
//! #[derive(Debug)] struct UserCreate;
//! #[derive(Debug)] struct UserUpdate;
//! #[derive(Debug)] enum UserAction {}
//! #[derive(Debug, thiserror::Error)] #[error("Err")] struct UserError;
//!
//! #[async_trait]
//! impl ActorService for User {
//!     type Id = u32; type Create = UserCreate; type Update = UserUpdate;
//!     type Action = UserAction; type ActionResult = (); type Error = UserError; type Entity = User;
//!
//!     async fn handle_create(&mut self, id: Option<u32>, _: UserCreate, _: &mut DynContext<Self::Id>) -> Result<u32, Self::Error> { self.id = id.unwrap_or(0); Ok(self.id) }
//!     async fn handle_get(&self, _: u32, _: &mut DynContext<Self::Id>) -> Result<Option<Self::Entity>, Self::Error> { Ok(Some(self.clone())) }
//!     async fn handle_update(&mut self, _: u32, _: UserUpdate, _: &mut DynContext<Self::Id>) -> Result<Self::Entity, Self::Error> { Ok(self.clone()) }
//!     async fn handle_delete(&mut self, _: u32, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_action(&mut self, _: Option<u32>, _: UserAction, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
//!     async fn handle_list(&mut self, _: &mut DynContext<Self::Id>) -> Result<Vec<User>, Self::Error> { Ok(vec![self.clone()]) }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut mock = MockClient::<User>::new();
//!     let client = mock.client();
//!
//!     // Simulate a downstream failure
//!     mock.expect_get(1)
//!         .return_err(FrameworkError::ActorClosed);
//!
//!     // Verify your code handles it gracefully
//!     let result = client.get(1).await;
//!     assert!(matches!(result, Err(FrameworkError::ActorClosed)));
//! }
//! ```
//!
//! ## Advanced: Test-Only Actions
//!
//! <details>
//! <summary><b>How to use Feature Flags for Testing</b></summary>
//!
//! Sometimes you need to inspect internal actor state for testing. Use a Cargo **feature flag** (`testing`)
//! instead of `#[cfg(test)]` so it works with integration tests.
//!
//! ```toml
//! [features]
//! testing = []
//! ```
//!
//! Then guard your test-only actions:
//!
//! ```rust,ignore
//! pub enum ProductAction {
//!     #[cfg(feature = "testing")]
//!     GetInternalState,
//! }
//! ```
//! </details>
//!
//! ## Mocking Utilities
//!
//! Use [`create_mock_client`] to get a client and a receiver, or use the fluent [`MockClient`] API.

use crate::client::ResourceClient;
use crate::error::FrameworkError;
use crate::message::ResourceRequest;
use crate::service::ActorService;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// =============================================================================
// EXPECTATION BUILDER API
// =============================================================================

/// Represents an expected request to the mock client.
///
/// This enum is used internally by `MockClient` to track what requests
/// are expected and what responses should be returned.
#[allow(dead_code)] // Future features: Update, Delete, Action expectations
enum Expectation<T: ActorService> {
    Get { id: T::Id, response: Result<Option<T::Entity>, FrameworkError> },
    Create { response: Result<T::Id, FrameworkError> },
    Update { id: T::Id, response: Result<T::Entity, FrameworkError> },
    Delete { id: T::Id, response: Result<(), FrameworkError> },
    Action { id: T::Id, response: Result<T::ActionResult, FrameworkError> },
}

/// A mock client with expectation tracking for fluent testing.
///
/// # Example
/// ```ignore
/// let mut mock = MockClient::<User>::new();
/// mock.expect_get("user_1".to_string()).return_ok(Some(user));
/// mock.expect_create().return_ok("user_2".to_string());
///
/// let client = mock.client();
/// // Use client in tests...
/// mock.verify(); // Ensures all expectations were met
/// ```
pub struct MockClient<T: ActorService> {
    client: ResourceClient<T>,
    expectations: Arc<Mutex<VecDeque<Expectation<T>>>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl<T: ActorService + Send + 'static> Default for MockClient<T>
where
    T::Id: Send,
    T::Create: Send,
    T::Update: Send,
    T::Action: Send,
    T::ActionResult: Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ActorService + Send + 'static> MockClient<T>
where
    T::Id: Send,
    T::Create: Send,
    T::Update: Send,
    T::Action: Send,
    T::ActionResult: Send,
{
    /// Creates a new mock client with no expectations.
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<ResourceRequest<T>>(100);
        let expectations = Arc::new(Mutex::new(VecDeque::new()));
        let expectations_clone = expectations.clone();

        // Spawn background task to handle requests
        let handle = tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                let mut exps = expectations_clone.lock().unwrap();
                let expectation = exps.pop_front();
                drop(exps); // Release lock before async operations

                match (request, expectation) {
                    (
                        ResourceRequest::Get { id: _, respond_to },
                        Some(Expectation::Get { id: _, response }),
                    ) => {
                        let _ = respond_to.send(response);
                    }
                    (
                        ResourceRequest::Create { params: _, respond_to, id: _ },
                        Some(Expectation::Create { response }),
                    ) => {
                        let _ = respond_to.send(response);
                    }
                    (
                        ResourceRequest::Update { id: _, update: _, respond_to },
                        Some(Expectation::Update { id: _, response }),
                    ) => {
                        let _ = respond_to.send(response);
                    }
                    (
                        ResourceRequest::Delete { id: _, respond_to },
                        Some(Expectation::Delete { id: _, response }),
                    ) => {
                        let _ = respond_to.send(response);
                    }
                    (
                        ResourceRequest::Action { id: _, action: _, respond_to },
                        Some(Expectation::Action { id: _, response }),
                    ) => {
                        let _ = respond_to.send(response);
                    }
                    _ => {
                        panic!("Unexpected request or expectation mismatch");
                    }
                }
            }
        });

        Self { client: ResourceClient::new(sender), expectations, _handle: handle }
    }

    /// Returns the client for use in tests.
    pub fn client(&self) -> ResourceClient<T> {
        self.client.clone()
    }

    /// Expects a `get` operation.
    pub fn expect_get(&mut self, id: T::Id) -> GetExpectationBuilder<T> {
        GetExpectationBuilder { id, expectations: self.expectations.clone() }
    }

    /// Expects a `create` operation.
    pub fn expect_create(&mut self) -> CreateExpectationBuilder<T> {
        CreateExpectationBuilder { expectations: self.expectations.clone() }
    }

    /// Expects an `action` operation.
    pub fn expect_action(&mut self, id: T::Id) -> ActionExpectationBuilder<T> {
        ActionExpectationBuilder { id, expectations: self.expectations.clone() }
    }

    /// Expects an `update` operation.
    pub fn expect_update(&mut self, id: T::Id) -> UpdateExpectationBuilder<T> {
        UpdateExpectationBuilder { id, expectations: self.expectations.clone() }
    }

    /// Expects a `delete` operation.
    pub fn expect_delete(&mut self, id: T::Id) -> DeleteExpectationBuilder<T> {
        DeleteExpectationBuilder { id, expectations: self.expectations.clone() }
    }

    /// Verifies that all expectations were met.
    pub fn verify(&self) {
        let exps = self.expectations.lock().unwrap();
        if !exps.is_empty() {
            panic!("Not all expectations were met. {} remaining", exps.len());
        }
    }
}

/// Builder for `get` expectations.
pub struct GetExpectationBuilder<T: ActorService> {
    id: T::Id,
    expectations: Arc<Mutex<VecDeque<Expectation<T>>>>,
}

impl<T: ActorService> GetExpectationBuilder<T> {
    /// Sets the expectation to return a successful result.
    pub fn return_ok(self, value: Option<T::Entity>) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Get { id: self.id, response: Ok(value) });
    }

    /// Sets the expectation to return an error.
    pub fn return_err(self, error: FrameworkError) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Get { id: self.id, response: Err(error) });
    }
}

/// Builder for `create` expectations.
pub struct CreateExpectationBuilder<T: ActorService> {
    expectations: Arc<Mutex<VecDeque<Expectation<T>>>>,
}

impl<T: ActorService> CreateExpectationBuilder<T> {
    /// Sets the expectation to return a successful result.
    pub fn return_ok(self, id: T::Id) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Create { response: Ok(id) });
    }

    /// Sets the expectation to return an error.
    pub fn return_err(self, error: FrameworkError) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Create { response: Err(error) });
    }
}

/// Builder for `action` expectations.
pub struct ActionExpectationBuilder<T: ActorService> {
    id: T::Id,
    expectations: Arc<Mutex<VecDeque<Expectation<T>>>>,
}

impl<T: ActorService> ActionExpectationBuilder<T> {
    /// Sets the expectation to return a successful result.
    pub fn return_ok(self, result: T::ActionResult) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Action { id: self.id, response: Ok(result) });
    }

    /// Sets the expectation to return an error.
    pub fn return_err(self, error: FrameworkError) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Action { id: self.id, response: Err(error) });
    }
}

/// Builder for `update` expectations.
pub struct UpdateExpectationBuilder<T: ActorService> {
    id: T::Id,
    expectations: Arc<Mutex<VecDeque<Expectation<T>>>>,
}

impl<T: ActorService> UpdateExpectationBuilder<T> {
    /// Sets the expectation to return a successful result.
    pub fn return_ok(self, entity: T::Entity) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Update { id: self.id, response: Ok(entity) });
    }

    /// Sets the expectation to return an error.
    pub fn return_err(self, error: FrameworkError) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Update { id: self.id, response: Err(error) });
    }
}

/// Builder for `delete` expectations.
pub struct DeleteExpectationBuilder<T: ActorService> {
    id: T::Id,
    expectations: Arc<Mutex<VecDeque<Expectation<T>>>>,
}

impl<T: ActorService> DeleteExpectationBuilder<T> {
    /// Sets the expectation to return a successful result.
    pub fn return_ok(self) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Delete { id: self.id, response: Ok(()) });
    }

    /// Sets the expectation to return an error.
    pub fn return_err(self, error: FrameworkError) {
        let mut exps = self.expectations.lock().unwrap();
        exps.push_back(Expectation::Delete { id: self.id, response: Err(error) });
    }
}

// =============================================================================
// LEGACY HELPERS (for backward compatibility)
// =============================================================================

/// Creates a mock client and a receiver for asserting requests.
///
/// # Testing Strategy
/// In unit/integration tests, we don't want to spin up a full `ResourceActor` if we are just
/// testing the *Client* logic (e.g., `OrderClient`).
///
/// Instead, we create a "Mock Client". This client sends messages to a channel we control (`receiver`).
/// We can then inspect the messages arriving on that channel and assert they are correct.
/// This allows us to simulate the Actor's behavior (success, failure, delays) deterministically.
///
/// **Note**: Consider using [`MockClient`] for a more fluent API.
pub fn create_mock_client<T: ActorService>(
    buffer_size: usize,
) -> (ResourceClient<T>, mpsc::Receiver<ResourceRequest<T>>) {
    let (sender, receiver) = mpsc::channel(buffer_size);
    (ResourceClient::new(sender), receiver)
}

/// Helper to verify that the next message is a Create request
pub async fn expect_create<T: ActorService>(
    receiver: &mut mpsc::Receiver<ResourceRequest<T>>,
) -> Option<(T::Create, tokio::sync::oneshot::Sender<Result<T::Id, FrameworkError>>)> {
    match receiver.recv().await {
        Some(ResourceRequest::Create { params, respond_to, id: _ }) => Some((params, respond_to)),
        _ => None,
    }
}

/// Helper to verify that the next message is a Get request
pub async fn expect_get<T: ActorService>(
    receiver: &mut mpsc::Receiver<ResourceRequest<T>>,
) -> Option<(T::Id, tokio::sync::oneshot::Sender<Result<Option<T::Entity>, FrameworkError>>)> {
    match receiver.recv().await {
        Some(ResourceRequest::Get { id, respond_to }) => Some((id, respond_to)),
        _ => None,
    }
}

/// Helper to verify that the next message is an Action request
pub async fn expect_action<T: ActorService>(
    receiver: &mut mpsc::Receiver<ResourceRequest<T>>,
) -> Option<(
    Option<T::Id>,
    T::Action,
    tokio::sync::oneshot::Sender<Result<T::ActionResult, FrameworkError>>,
)> {
    match receiver.recv().await {
        Some(ResourceRequest::Action { id, action, respond_to }) => Some((id, action, respond_to)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::*;
    use crate::service::ActorService;
    use crate::{Context, DynContext};
    use async_trait::async_trait;

    #[derive(Clone, Debug, PartialEq)]
    struct User {
        id: u32,
        name: String,
        email: String,
    }

    #[derive(Debug)]
    struct UserCreate {
        name: String,
        email: String,
    }

    #[derive(Debug)]
    struct UserUpdate;

    #[derive(Debug)]
    enum UserAction {}

    #[derive(Debug, thiserror::Error)]
    #[error("User error")]
    struct UserError;

    #[async_trait]
    impl ActorService for User {
        type Id = u32;
        type Create = UserCreate;
        type Update = UserUpdate;
        type Action = UserAction;
        type ActionResult = ();
        type Error = UserError;
        type Entity = User;

        fn from_create_params(id: u32, params: UserCreate) -> Result<Self, Self::Error> {
            Ok(Self { id, name: params.name, email: params.email })
        }

        async fn handle_create(
            &mut self,
            id: Option<Self::Id>,
            params: Self::Create,
            _ctx: &mut DynContext<Self::Id>,
        ) -> Result<Self::Id, Self::Error> {
            Ok(id.unwrap_or(0))
        }

        async fn handle_get(
            &self,
            id: Self::Id,
            _ctx: &mut DynContext<Self::Id>,
        ) -> Result<Option<Self::Entity>, Self::Error> {
            Ok(None)
        }

        async fn handle_update(
            &mut self,
            _id: Self::Id,
            _update: Self::Update,
            _ctx: &mut DynContext<Self::Id>,
        ) -> Result<Self::Entity, Self::Error> {
            Ok(self.clone())
        }

        async fn handle_delete(
            &mut self,
            _id: Self::Id,
            _ctx: &mut DynContext<Self::Id>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn handle_action(
            &mut self,
            _id: Option<Self::Id>,
            _action: Self::Action,
            _ctx: &mut DynContext<Self::Id>,
        ) -> Result<Self::ActionResult, Self::Error> {
            Ok(())
        }

        async fn handle_list(
            &mut self,
            _ctx: &mut DynContext<Self::Id>,
        ) -> Result<Vec<Self::Entity>, Self::Error> {
            Ok(vec![])
        }
    }

    impl User {
        fn new(id: u32, email: &str) -> Self {
            Self { id, name: "Test User".to_string(), email: email.to_string() }
        }
    }

    // Note: MockClient tests rely on ActorService for types, but intercept messages.

    #[tokio::test]
    async fn test_mock_client() {
        let (client, mut receiver) = create_mock_client::<User>(10);

        // Test logic uses tokio::spawn directly, bypassing client.spawn.
        let create_task = tokio::spawn(async move {
            let user =
                UserCreate { name: "Test".to_string(), email: "test@example.com".to_string() };
            client.create(user).await
        });

        let (payload, responder) =
            expect_create(&mut receiver).await.expect("Expected Create request");
        assert_eq!(payload.name, "Test");
        responder.send(Ok(1)).unwrap();

        let result = create_task.await.unwrap();
        assert!(matches!(result, Ok(id) if id == 1));
    }

    #[tokio::test]
    async fn test_mock_client_with_expectations() {
        // Create mock with fluent expectation API
        let mut mock = MockClient::<User>::new();

        // Set up expectations
        mock.expect_create().return_ok(1);
        mock.expect_get(1).return_ok(Some(User::new(1, "test@example.com")));

        let client = mock.client();

        // Execute operations
        let user = UserCreate { name: "Test".to_string(), email: "test@example.com".to_string() };
        let id = client.create(user).await.unwrap();
        assert_eq!(id, 1);

        let fetched = client.get(1).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().email, "test@example.com");

        // Verify all expectations were met
        mock.verify();
    }
}
