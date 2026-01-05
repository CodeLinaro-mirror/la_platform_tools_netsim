//! # Generic Actor Server
//!
//! This module defines the `ResourceActor`, the core component that manages the lifecycle
//! and state of entities. It implements the "Server" side of the Actor Model, processing
//! messages sequentially and ensuring exclusive access to the entity store.

use crate::client::ResourceClient;
use crate::entity::{ActorContext, ActorEntity, StreamMessage};
use crate::error::FrameworkError;
use crate::message::ResourceRequest;
use crate::runtime::Runtime;
use log::error;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{StreamExt, StreamMap};
// use tracing::{debug, info, warn};

/// The generic actor that manages a collection of entities.
///
/// # Architecture Note
/// This struct is the "Server" half of the actor. It owns the state (`store`) and
/// the receiver end of the channel.
///
/// **Concurrency Model**:
/// Even though we might have 1000 `ResourceActor` instances running, each one
/// processes its own messages *sequentially* in a loop. This means we don't need
/// `Mutex` or `RwLock` for the `store`! The "Actor Model" gives us safety through
/// exclusive ownership of state within the task.
/// ## ResourceActor
///
/// The `ResourceActor<T>` struct is the *server* side of the framework. It owns the in‑memory store for a given entity type `T: ActorEntity` and processes all incoming `ResourceRequest<T>` messages sequentially. Each actor runs in its own Tokio task, guaranteeing exclusive access to its state without any locking.
///
/// * **Concurrency model** – each actor processes one message at a time, eliminating data races.
/// * **Context injection** – a user‑provided `Context` is passed to every lifecycle hook, enabling dependency injection.
/// * **Uniform API** – works with any entity that implements `ActorEntity`, providing a generic CRUD + Action implementation.
///
/// # Usage Pattern
///
/// The canonical way to create and wire actors is:
///
/// 1.  **Create**: Call `ResourceActor::new()` to get the `actor` (server) and `client` (interface).
/// 2.  **Wire**: Pass dependencies (other clients) into `actor.run(context)`.
/// 3.  **Run**: Spawn the actor's run loop in a background task.
///
/// ```rust
/// use actor_framework::{ActorEntity, Runtime, ResourceActor};
/// use async_trait::async_trait;
///
/// // Minimal Entity Definition
/// #[derive(Clone, Debug)] struct MyEntity { id: u32 }
/// #[derive(Debug)] struct MyCreate;
/// #[derive(Debug)] struct MyUpdate;
/// #[derive(Debug)] enum MyAction {}
/// #[derive(Debug)] struct MyError(String);
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
/// }
/// impl std::error::Error for MyError {}
/// impl From<String> for MyError { fn from(s: String) -> Self { MyError(s) } }
///
/// #[async_trait]
/// impl ActorEntity for MyEntity {
///     type Id = u32;
///     type Create = MyCreate;
///     type Update = MyUpdate;
///     type Action = MyAction;
///     type ActionResult = ();
///     type Context = (); // No dependencies in this example
///     type Error = MyError;
///     type ListResponse = Vec<MyEntity>;
///
///     fn from_create_params(id: u32, _: MyCreate) -> Result<Self, Self::Error> {
///         Ok(Self { id })
///     }
///     async fn on_update(&mut self, _: MyUpdate, _: &mut Self::Context, _: &mut impl Runtime) -> Result<(), Self::Error> { Ok(()) }
///     async fn handle_action(&mut self, _: MyAction, _: &mut Self::Context, _: &mut impl Runtime) -> Result<(), Self::Error> { Ok(()) }
///     fn on_list(_: &std::collections::HashMap<Self::Id, Self>, _: &mut Self::Context, _: &mut impl Runtime) -> Self::ListResponse { vec![] }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     // 1. Create
///     let (actor, client) = ResourceActor::<MyEntity>::new(10);
///
///     // 2. Wire & Run
///     tokio::spawn(actor.run(()));
///
///     // 3. Use
///     let _ = client.create(MyCreate).await;
/// }
/// ```
///
/// # Implementation Details
///
/// The actor maintains an internal `HashMap` (`store`) mapping IDs to entities and a `u32` counter (`next_id`) for ID generation.
///
/// ## Operations
///
/// * **Create**:
///     1. Generates a new ID using the internal `next_id` counter (incrementing it).
///     2. Converts the `u32` ID to `T::Id`.
///     3. Calls `T::from_create_params` to instantiate the entity.
///     4. Calls the `on_create` lifecycle hook.
///     5. Inserts the new entity into the `store`.
///     6. Returns the new ID.
///
/// * **Get**:
///     1. Looks up the entity in the `store` by ID.
///     2. Returns a clone of the entity if found, or `None`.
///
/// * **Update**:
///     1. Looks up the entity in the `store` (mutable access).
///     2. Calls the `on_update` lifecycle hook with the update DTO.
///     3. The entity modifies its own state within the hook.
///     4. Returns the updated entity state.
///
/// * **Delete**:
///     1. Looks up the entity in the `store`.
///     2. Calls the `on_delete` lifecycle hook.
///     3. Removes the entity from the `store`.
///
/// * **Action**:
///     1. Looks up the entity in the `store` (mutable access).
///     2. Calls the `handle_action` hook with the custom action enum.
///     3. Returns the result of the action.
pub struct ResourceActor<T: ActorEntity> {
    receiver: mpsc::Receiver<ResourceRequest<T>>,
    store: HashMap<T::Id, T>,
    next_id: u32,
    shutdown_rx: oneshot::Receiver<()>,
    runtime: crate::runtime::StandardRuntime,
}

impl<T: ActorEntity> ResourceActor<T> {
    /// Creates a new `ResourceActor` and its associated `ResourceClient`.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - The capacity of the MPSC channel. If the channel is full,
    ///   calls to the client will wait until there is space.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// 1. The `ResourceActor` instance (the server), which must be run via `.run()`.
    /// 2. The `ResourceClient` instance, which can be cloned and shared to send requests.
    pub fn new(buffer_size: usize) -> (Self, ResourceClient<T>) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let (runtime, shutdown_rx) = crate::runtime::StandardRuntime::new();
        let actor = Self { receiver, store: HashMap::new(), next_id: 1, shutdown_rx, runtime };
        let client = ResourceClient::new(sender);
        (actor, client)
    }

    /// Runs the actor's event loop, processing messages until the channel closes.
    ///
    /// # Context Injection
    /// The `context` argument is injected into every entity hook. This allows entities
    /// to access external dependencies (like other clients) that were created *after*
    /// the actor was instantiated but *before* the loop started.
    pub async fn run(mut self, mut context: T::Context) {
        context.on_start(&mut self.runtime).await;

        loop {
            // Move out of select! to avoid borrow conflicts
            let stream_fut = self.runtime.streams.next();
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    Self::handle_message(&mut self.store, &mut self.next_id, msg, &mut context, &mut self.runtime).await;
                }
                _ = self.runtime.interval.tick() => {
                    context.on_tick(&mut self.runtime).await;
                }
                Some((id, msg_opt)) = stream_fut => {
                    Self::handle_stream_event(&mut self.store, id, msg_opt, &mut context, &mut self.runtime).await;
                }
                _ = &mut self.shutdown_rx => {
                    Self::handle_shutdown(&mut context).await;
                    break;
                }
            }
        }
    }

    async fn handle_stream_event(
        store: &mut HashMap<T::Id, T>,
        id: usize,
        msg_opt: Option<StreamMessage>,
        context: &mut T::Context,
        runtime: &mut impl Runtime,
    ) {
        match msg_opt {
            Some(msg) => context.on_stream(id, msg, runtime).await,
            Option::None => {
                // Stream closed
                if let Ok(true) = context.on_stream_closed(id).await {
                    // Delete entity
                    let entity_id = T::Id::from(id as u32);
                    if let Some(item) = store.get(&entity_id) {
                        if let Err(_e) = item.on_delete(context, runtime).await {}
                        store.remove(&entity_id);
                    }
                }
            }
        }
    }

    async fn handle_shutdown(context: &mut T::Context) {
        if let Err(_e) = context.on_shutdown().await {
            error!("on_shutdown failed: {}", _e);
        }
    }

    async fn handle_message(
        store: &mut HashMap<T::Id, T>,
        next_id: &mut u32,
        msg: ResourceRequest<T>,
        context: &mut T::Context,
        runtime: &mut impl Runtime,
    ) {
        match msg {
            ResourceRequest::Create { params, respond_to } => {
                let id = T::Id::from(*next_id);
                *next_id += 1;

                match T::from_create_params(id.clone(), params) {
                    Ok(mut item) => {
                        // Await the async hook
                        if let Err(e) = item.on_create(context, runtime).await {
                            let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                            return;
                        }
                        store.insert(id.clone(), item);
                        let _ = respond_to.send(Ok(id));
                    }
                    Err(e) => {
                        let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                    }
                }
            }
            ResourceRequest::Get { id, respond_to } => {
                let item = store.get(&id).cloned();
                let _found = item.is_some();
                let _ = respond_to.send(Ok(item));
            }
            ResourceRequest::Update { id, update, respond_to } => {
                if let Some(item) = store.get_mut(&id) {
                    // Await the async hook
                    if let Err(e) = item.on_update(update, context, runtime).await {
                        let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                        return;
                    }
                    let _ = respond_to.send(Ok(item.clone()));
                } else {
                    let _ = respond_to.send(Err(FrameworkError::NotFound(id.to_string())));
                }
            }
            ResourceRequest::Delete { id, respond_to } => {
                if let Some(item) = store.get(&id) {
                    // Await the async hook
                    if let Err(e) = item.on_delete(context, runtime).await {
                        let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                        return;
                    }
                    store.remove(&id);
                    let _ = respond_to.send(Ok(()));
                } else {
                    let _ = respond_to.send(Err(FrameworkError::NotFound(id.to_string())));
                }
            }
            ResourceRequest::Action { id, action, respond_to } => {
                if let Some(item) = store.get_mut(&id) {
                    // Await the async hook
                    let result = item
                        .handle_action(action, context, runtime)
                        .await
                        .map_err(|e| FrameworkError::EntityError(Box::new(e)));
                    match &result {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                    let _ = respond_to.send(result);
                } else {
                    let _ = respond_to.send(Err(FrameworkError::NotFound(id.to_string())));
                }
            }
            ResourceRequest::List { respond_to } => {
                let response = T::on_list(&store, context, runtime);
                let _ = respond_to.send(Ok(response));
            }
        }
    }
}
