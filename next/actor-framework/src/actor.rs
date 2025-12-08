//! # Generic Actor Server
//!
//! This module defines the `ResourceActor`, the core component that manages the lifecycle
//! and state of entities. It implements the "Server" side of the Actor Model, processing
//! messages sequentially and ensuring exclusive access to the entity store.

use crate::client::ResourceClient;
use crate::entity::ActorEntity;
use crate::error::FrameworkError;
use crate::message::ResourceRequest;
use std::collections::HashMap;
use tokio::sync::mpsc;
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
/// use actor_framework::{ActorEntity, ResourceActor};
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
///     fn from_create_params(id: u32, _: MyCreate) -> Result<Self, Self::Error> { Ok(Self { id }) }
///     async fn on_update(&mut self, _: MyUpdate, _: &mut ()) -> Result<(), Self::Error> { Ok(()) }
///     async fn handle_action(&mut self, _: MyAction, _: &mut ()) -> Result<(), Self::Error> { Ok(()) }
///     fn on_list(_: &std::collections::HashMap<Self::Id, Self>, _: &mut ()) -> Self::ListResponse { vec![] }
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
        let actor = Self { receiver, store: HashMap::new(), next_id: 1 };
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
        // Extract just the type name (e.g., "User" instead of "actor_recipe::model::user::User")
        let _entity_type = std::any::type_name::<T>().split("::").last().unwrap_or("Unknown");
        // info!(entity_type, "Actor started");

        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ResourceRequest::Create { params, respond_to } => {
                    // debug!(entity_type, ?params, "Create");
                    let id = T::Id::from(self.next_id);
                    self.next_id += 1;

                    match T::from_create_params(id.clone(), params) {
                        Ok(mut item) => {
                            // Await the async hook
                            if let Err(e) = item.on_create(&mut context).await {
                                // warn!(entity_type, error = %e, "on_create failed");
                                let _ =
                                    respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                                continue;
                            }
                            self.store.insert(id.clone(), item);
                            // info!(entity_type, %id, size = self.store.len(), "Created");
                            let _ = respond_to.send(Ok(id));
                        }
                        Err(e) => {
                            // warn!(entity_type, error = %e, "Create failed");
                            let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                        }
                    }
                }
                ResourceRequest::Get { id, respond_to } => {
                    let item = self.store.get(&id).cloned();
                    let _found = item.is_some();
                    // debug!(entity_type, %id, found, "Get");
                    let _ = respond_to.send(Ok(item));
                }
                ResourceRequest::Update { id, update, respond_to } => {
                    // debug!(entity_type, %id, ?update, "Update");
                    if let Some(item) = self.store.get_mut(&id) {
                        // Await the async hook
                        if let Err(e) = item.on_update(update, &mut context).await {
                            // warn!(entity_type, %id, error = %e, "Update failed");
                            let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                            continue;
                        }
                        // info!(entity_type, %id, "Updated");
                        let _ = respond_to.send(Ok(item.clone()));
                    } else {
                        // warn!(entity_type, %id, "Not found");
                        let _ = respond_to.send(Err(FrameworkError::NotFound(id.to_string())));
                    }
                }
                ResourceRequest::Delete { id, respond_to } => {
                    // debug!(entity_type, %id, "Delete");
                    if let Some(item) = self.store.get(&id) {
                        // Await the async hook
                        if let Err(e) = item.on_delete(&mut context).await {
                            // warn!(entity_type, %id, error = %e, "on_delete failed");
                            let _ = respond_to.send(Err(FrameworkError::EntityError(Box::new(e))));
                            continue;
                        }
                        self.store.remove(&id);
                        // info!(entity_type, %id, size = self.store.len(), "Deleted");
                        let _ = respond_to.send(Ok(()));
                    } else {
                        // warn!(entity_type, %id, "Not found");
                        let _ = respond_to.send(Err(FrameworkError::NotFound(id.to_string())));
                    }
                }
                ResourceRequest::Action { id, action, respond_to } => {
                    // debug!(entity_type, %id, ?action, "Action");
                    if let Some(item) = self.store.get_mut(&id) {
                        // Await the async hook
                        let result = item
                            .handle_action(action, &mut context)
                            .await
                            .map_err(|e| FrameworkError::EntityError(Box::new(e)));
                        match &result {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                        let _ = respond_to.send(result);
                    } else {
                        // warn!(entity_type, %id, "Not found");
                        let _ = respond_to.send(Err(FrameworkError::NotFound(id.to_string())));
                    }
                }
                ResourceRequest::List { respond_to } => {
                    let response = T::on_list(&self.store, &mut context);
                    let _ = respond_to.send(Ok(response));
                }
            }
        }

        // info!(entity_type, size = self.store.len(), "Shutdown");
    }
}
