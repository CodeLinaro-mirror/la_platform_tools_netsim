//! # Generic Actor Server
//!
//! This module defines the `ResourceActor`, the core component that manages the
//! lifecycle and state of resources. It implements the "Server" side of the
//! Actor Model, processing messages sequentially and ensuring exclusive access
//! to the resource store.

use log::error;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

use crate::{
    client::ResourceClient, context::FrameworkContext, error::FrameworkError,
    message::ResourceRequest, ActorLifecycle, ActorService, DynContext, StreamMessage,
};
// use tracing::{debug, info, warn};

/// The generic actor that manages a collection of resources.
///
/// # Architecture Note
/// This struct is the "Server" half of the actor.
///
/// **Concurrency Model**:
/// Even though we might have 1000 `ResourceActor` instances running, each one
/// processes its own messages *sequentially* in a loop.
/// ## ResourceActor
///
/// The `ResourceActor<T>` struct is the *server* side of the framework. It
/// delegates operations to the underlying service `T: ActorService`.
///
/// * **Concurrency model** – each actor processes one message at a time,
///   eliminating data races.
/// * **Context injection** – a user‑provided `Context` is passed to every
///   lifecycle hook.
/// * **Uniform API** – works with any resource that implements `ActorService`.
///
/// # Usage Pattern
///
/// The canonical way to create and wire actors is:
///
/// 1. **Create**: Call `ResourceActor::new()` to get the `actor` (server) and
///    `client` (interface).
/// 2. **Wire**: Pass dependencies (other clients) into `actor.run(context)`.
/// 3. **Run**: Spawn the actor's run loop in a background task.
///
/// ```rust
/// use actor_framework::{
///     ActorLifecycle, ActorService, BoxStream, Context, DynContext, ResourceActor,
/// };
///
/// // Minimal Actor Definition
/// #[derive(Clone, Debug)]
/// struct MyActor {
///     id: u32,
/// }
/// #[derive(Debug)]
/// struct MyCreate;
/// #[derive(Debug)]
/// struct MyUpdate;
/// #[derive(Debug)]
/// enum MyAction {}
/// #[derive(Debug)]
/// struct MyError(String);
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
/// impl std::error::Error for MyError {}
/// impl From<String> for MyError {
///     fn from(s: String) -> Self {
///         MyError(s)
///     }
/// }
///
/// impl ActorService for MyActor {
///     type Id = u32;
///     type Create = MyCreate;
///     type Update = MyUpdate;
///     type Action = MyAction;
///     type ActionResult = ();
///     type Error = MyError;
///     type Entity = MyActor;
///
///     async fn handle_create(
///         &mut self,
///         id: Option<u32>,
///         _: MyCreate,
///         _: &mut DynContext<Self>,
///     ) -> Result<u32, Self::Error> {
///         self.id = id.unwrap_or(0);
///         Ok(self.id)
///     }
///     async fn handle_get(
///         &self,
///         _: u32,
///         _: &mut DynContext<Self>,
///     ) -> Result<Option<Self::Entity>, Self::Error> {
///         Ok(Some(self.clone()))
///     }
///     async fn handle_update(
///         &mut self,
///         _: u32,
///         _: MyUpdate,
///         _: &mut DynContext<Self>,
///     ) -> Result<Self::Entity, Self::Error> {
///         Ok(self.clone())
///     }
///     async fn handle_delete(
///         &mut self,
///         _: u32,
///         _: &mut DynContext<Self>,
///     ) -> Result<(), Self::Error> {
///         Ok(())
///     }
///     async fn handle_action(
///         &mut self,
///         _: Option<u32>,
///         _: MyAction,
///         _: &mut DynContext<Self>,
///     ) -> Result<(), Self::Error> {
///         Ok(())
///     }
///     async fn handle_list(
///         &mut self,
///         _: &mut DynContext<Self>,
///     ) -> Result<Vec<MyActor>, Self::Error> {
///         Ok(vec![self.clone()])
///     }
/// }
///
/// impl ActorLifecycle for MyActor {
///     async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {}
///     async fn on_tick(&mut self, _ctx: &mut DynContext<Self>) {}
///     async fn on_stream(&mut self, _id: u32, _msg: bytes::Bytes, _ctx: &mut DynContext<Self>) {}
///     async fn on_stream_closed(&mut self, _id: u32, _ctx: &mut DynContext<Self>) {}
///     async fn on_task_closed(&mut self, _id: u32, _ctx: &mut DynContext<Self>) {}
///     async fn on_shutdown(&mut self) {}
/// }
///
/// #[tokio::main]
/// async fn main() {
///     // 1. Create
///     let (actor, client) = ResourceActor::<MyActor>::new(10);
///
///     // 2. Wire & Run
///     let my_actor_impl = MyActor { id: 0 };
///     tokio::spawn(actor.run(my_actor_impl));
///
///     // 3. Use
///     let _ = client.create(MyCreate).await;
/// }
/// ```
pub struct ResourceActor<T: ActorService> {
    receiver: mpsc::Receiver<ResourceRequest<T>>,
    shutdown_rx: oneshot::Receiver<()>,
    ctx: FrameworkContext<T>,
}

impl<T: ActorService + ActorLifecycle> ResourceActor<T> {
    /// Creates a new `ResourceActor` and its associated `ResourceClient`.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - The capacity of the MPSC channel. If the channel is
    ///   full, calls to the client will wait until there is space.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// 1. The `ResourceActor` instance (the server), which must be run via
    ///    `.run(actor)`.
    /// 2. The `ResourceClient` instance, which can be cloned and shared to send
    ///    requests.
    pub fn new(channel_size: usize) -> (Self, ResourceClient<T>) {
        let (sender, receiver) = mpsc::channel(channel_size);
        let (ctx, shutdown_rx) = FrameworkContext::new();
        let actor = Self { receiver, shutdown_rx, ctx };
        let client = ResourceClient::new(sender);
        (actor, client)
    }

    /// Runs the actor's event loop, processing messages until the channel
    /// closes.
    ///
    /// # Context Injection
    /// The `context` argument is injected into every service hook. This allows
    /// services to access external dependencies (like other clients) that
    /// were created *after* the actor was instantiated but *before* the
    /// loop started.
    pub async fn run(mut self, mut actor: T) {
        actor.on_start(&mut self.ctx).await;

        loop {
            // Move out of select! to avoid borrow conflicts
            let stream_fut = self.ctx.streams.next();
            // We need to poll the timers
            // If the delay queue is empty, peek() returns None, which is fine.
            let timer_fut = self.ctx.timers.next();

            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    Self::handle_message(&mut actor, msg, &mut self.ctx).await;
                }
                _ = self.ctx.interval.tick() => {
                    actor.on_tick(&mut self.ctx).await;
                }
                Some((id, msg_opt)) = stream_fut => {
                    Self::handle_stream_event(&mut actor, id, msg_opt, &mut self.ctx).await;
                }
                Some(expired) = timer_fut => {
                    let task = expired.into_inner();
                    task(&mut actor, &mut self.ctx);
                }
                Some(res) = self.ctx.tasks.join_next() => {
                    match res {
                        Ok(id) => Self::handle_task_closed(&mut actor, id, &mut self.ctx).await,
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
        actor: &mut T,
        id: T::Id,
        msg_opt: Option<StreamMessage>,
        ctx: &mut DynContext<T>,
    ) {
        match msg_opt {
            Some(msg) => actor.on_stream(id, msg, ctx).await,
            Option::None => actor.on_stream_closed(id, ctx).await,
        }
    }

    async fn handle_task_closed(actor: &mut T, id: T::Id, ctx: &mut DynContext<T>) {
        actor.on_task_closed(id, ctx).await;
    }

    async fn handle_shutdown(actor: &mut T) {
        actor.on_shutdown().await;
    }

    async fn handle_message(actor: &mut T, msg: ResourceRequest<T>, ctx: &mut DynContext<T>) {
        match msg {
            ResourceRequest::Create { params, id, respond_to } => {
                // Pass the optional ID to the service handle_create method
                let result = actor
                    .handle_create(id, params, ctx)
                    .await
                    .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
                let _ = respond_to.send(result);
            }
            ResourceRequest::Get { id, respond_to } => {
                let result = actor
                    .handle_get(id, ctx)
                    .await
                    .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
                let _ = respond_to.send(result);
            }
            ResourceRequest::Update { id, update, respond_to } => {
                let result = actor
                    .handle_update(id, update, ctx)
                    .await
                    .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
                let _ = respond_to.send(result);
            }
            ResourceRequest::Delete { id, respond_to } => {
                let result = actor
                    .handle_delete(id, ctx)
                    .await
                    .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
                let _ = respond_to.send(result);
            }
            ResourceRequest::Action { id, action, respond_to } => {
                let result = actor
                    .handle_action(id, action, ctx)
                    .await
                    .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
                let _ = respond_to.send(result);
            }
            ResourceRequest::List { respond_to } => {
                let result = actor
                    .handle_list(ctx)
                    .await
                    .map_err(|e| FrameworkError::ServiceError(Box::new(e)));
                let _ = respond_to.send(result);
            }
            ResourceRequest::Shutdown { respond_to } => {
                ctx.shutdown();
                let _ = respond_to.send(Ok(()));
            }
        }
    }
}
