//! # Generic Actor Server
//!
//! This module defines the `ResourceActor`, the core component that manages the lifecycle
//! and state of resources. It implements the "Server" side of the Actor Model, processing
//! messages sequentially and ensuring exclusive access to the resource store.

use crate::client::ResourceClient;
use crate::context::FrameworkContext;
use crate::error::FrameworkError;
use crate::lifecycle::ActorLifecycle;
use crate::message::ResourceRequest;
use crate::service::{ActorService, StreamMessage};
use crate::Context;
use log::error;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;
// use tracing::{debug, info, warn};

/// The generic actor that manages a collection of resources.
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
/// The `ResourceActor<T>` struct is the *server* side of the framework. It owns the in‑memory store for a given resource type `T: ActorService` and processes all incoming `ResourceRequest<T>` messages sequentially. Each actor runs in its own Tokio task, guaranteeing exclusive access to its state without any locking.
///
/// * **Concurrency model** – each actor processes one message at a time, eliminating data races.
/// * **Context injection** – a user‑provided `Context` is passed to every lifecycle hook, enabling dependency injection.
/// * **Uniform API** – works with any resource that implements `ActorService`, providing a generic CRUD + Action implementation.
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
/// use actor_framework::{ActorService, Context, ResourceActor};
/// use async_trait::async_trait;
///
/// // Minimal Service Definition
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
/// impl ActorService for MyEntity {
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
///     async fn on_update(&mut self, _: MyUpdate, _: &mut Self::Context, _: &mut impl Context) -> Result<(), Self::Error> { Ok(()) }
///     async fn handle_action(&mut self, _: MyAction, _: &mut Self::Context, _: &mut impl Context) -> Result<(), Self::Error> { Ok(()) }
///     fn on_list(_: &std::collections::HashMap<Self::Id, Self>, _: &mut Self::Context, _: &mut impl Context) -> Self::ListResponse { vec![] }
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
/// The actor maintains an internal `HashMap` (`store`) mapping IDs to resources and a `u32` counter (`next_id`) for ID generation.
///
/// ## Operations
///
/// * **Create**:
///     1. Generates a new ID using the internal `next_id` counter (incrementing it).
///     2. Converts the `u32` ID to `T::Id`.
///     3. Calls `T::from_create_params` to instantiate the resource.
///     4. Calls the `on_create` lifecycle hook.
///     5. Inserts the new resource into the `store`.
///     6. Returns the new ID.
///
/// * **Get**:
///     1. Looks up the resource in the `store` by ID.
///     2. Returns a clone of the resource if found, or `None`.
///
/// * **Update**:
///     1. Looks up the resource in the `store` (mutable access).
///     2. Calls the `on_update` lifecycle hook with the update DTO.
///     3. The resource modifies its own state within the hook.
///     4. Returns the updated resource state.
///
/// * **Delete**:
///     1. Looks up the resource in the `store`.
///     2. Calls the `on_delete` lifecycle hook.
///     3. Removes the resource from the `store`.
///
/// * **Action**:
///     1. Looks up the resource in the `store` (mutable access).
///     2. Calls the `handle_action` hook with the custom action enum.
///     3. Returns the result of the action.
pub struct ResourceActor<T: ActorService> {
    receiver: mpsc::Receiver<ResourceRequest<T>>,
    store: HashMap<T::Id, T>,
    next_id: u32,
    shutdown_rx: oneshot::Receiver<()>,
    ctx: FrameworkContext,
}

impl<T: ActorService> ResourceActor<T> {
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
    pub fn new(channel_size: usize) -> (Self, ResourceClient<T>) {
        let (sender, receiver) = mpsc::channel(channel_size);
        let (ctx, shutdown_rx) = FrameworkContext::new();
        let actor = Self { receiver, store: HashMap::new(), next_id: 1, shutdown_rx, ctx };
        let client = ResourceClient::new(sender);
        (actor, client)
    }

    /// Runs the actor's event loop, processing messages until the channel closes.
    ///
    /// # Context Injection
    /// The `context` argument is injected into every service hook. This allows services
    /// to access external dependencies (like other clients) that were created *after*
    /// the actor was instantiated but *before* the loop started.
    pub async fn run(mut self, mut actor: T::Context) {
        actor.on_start(&mut self.ctx).await;

        loop {
            // Move out of select! to avoid borrow conflicts
            let stream_fut = self.ctx.streams.next();
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    Self::handle_message(&mut self.store, &mut self.next_id, msg, &mut actor, &mut self.ctx).await;
                }
                _ = self.ctx.interval.tick() => {
                    actor.on_tick(&mut self.ctx).await;
                }
                Some((id, msg_opt)) = stream_fut => {
                    Self::handle_stream_event(&mut self.store, id, msg_opt, &mut actor, &mut self.ctx).await;
                }
                Some(res) = self.ctx.tasks.join_next() => {
                    match res {
                        Ok(id) => Self::handle_task_closed(&mut self.store, id, &mut actor, &mut self.ctx).await,
                        Err(e) => error!("Monitored task failed: {e}"),
                    }
                }
                _ = &mut self.shutdown_rx => {
                    Self::handle_shutdown(&mut actor).await;
                    break;
                }
            }
        }
    }

    async fn handle_stream_event(
        store: &mut HashMap<T::Id, T>,
        id: usize,
        msg_opt: Option<StreamMessage>,
        actor: &mut T::Context,
        ctx: &mut impl Context,
    ) {
        match msg_opt {
            Some(msg) => actor.on_stream(id, msg, ctx).await,
            Option::None => {
                // Stream closed
                if let Ok(true) = actor.on_stream_closed(id).await {
                    // Delete entity
                    let entity_id = T::Id::from(id as u32);
                    if let Some(item) = store.get(&entity_id) {
                        if let Err(_e) = item.on_delete(actor, ctx).await {}
                        store.remove(&entity_id);
                    }
                }
            }
        }
    }

    async fn handle_task_closed(
        store: &mut HashMap<T::Id, T>,
        id: usize,
        actor: &mut T::Context,
        ctx: &mut impl Context,
    ) {
        if let Ok(true) = actor.on_task_closed(id).await {
            // Delete entity
            let entity_id = T::Id::from(id as u32);
            if let Some(item) = store.get(&entity_id) {
                if let Err(_e) = item.on_delete(actor, ctx).await {}
                store.remove(&entity_id);
            }
        }
    }

    async fn handle_shutdown(actor: &mut T::Context) {
        if let Err(_e) = actor.on_shutdown().await {
            error!("on_shutdown failed: {}", _e);
        }
    }

    async fn handle_message(
        store: &mut HashMap<T::Id, T>,
        next_id: &mut u32,
        msg: ResourceRequest<T>,
        actor: &mut T::Context,
        ctx: &mut impl Context,
    ) {
        match msg {
            ResourceRequest::Create { params, respond_to } => {
                let id = T::Id::from(*next_id);
                *next_id += 1;

                match T::from_create_params(id.clone(), params) {
                    Ok(mut item) => {
                        // Await the async hook
                        if let Err(e) = item.on_create(actor, ctx).await {
                            let _ = respond_to.send(Err(FrameworkError::ServiceError(Box::new(e))));
                            return;
                        }
                        store.insert(id.clone(), item);
                        let _ = respond_to.send(Ok(id));
                    }
                    Err(e) => {
                        let _ = respond_to.send(Err(FrameworkError::ServiceError(Box::new(e))));
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
                    if let Err(e) = item.on_update(update, actor, ctx).await {
                        let _ = respond_to.send(Err(FrameworkError::ServiceError(Box::new(e))));
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
                    if let Err(e) = item.on_delete(actor, ctx).await {
                        let _ = respond_to.send(Err(FrameworkError::ServiceError(Box::new(e))));
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
                        .handle_action(action, actor, ctx)
                        .await
                        .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
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
                let response = T::on_list(&store, actor, ctx);
                let _ = respond_to.send(Ok(response));
            }
        }
    }
}
