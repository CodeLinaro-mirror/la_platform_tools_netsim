//! # Generic Messages
//!
//! This module defines the generic message types used for communication between
//! the `ResourceClient` and `ResourceActor`.

use tokio::sync::oneshot;

use crate::{error::FrameworkError, service::ActorService};

/// Type alias for the one-shot response channel used by actors.
pub type Response<T> = oneshot::Sender<Result<T, FrameworkError>>;

/// Internal message type sent to the actor to request operations.
///
/// # Resource-Oriented Architecture
/// This enum implements a **Resource-Oriented** design pattern where each actor
/// manages a specific type of resource (the [`ActorService`]). Instead of
/// defining ad-hoc messages for every operation, we standardize around a set of
/// lifecycle operations that apply to almost any persistent resource.
///
/// # The CRUD Pattern
/// The variants of this enum map directly to standard **CRUD** (Create, Read,
/// Update, Delete) operations, plus a custom `Action` variant for
/// resource-specific logic that doesn't fit the CRUD model.
///
/// - **Create**: Lifecycle start. Uses [`ActorService::Create`] to initialize a
///   new resource.
/// - **Get (Read)**: Retrieval. Fetches the current state of the resource by
///   ID.
/// - **Update**: State mutation. Uses [`ActorService::Update`] to modify an
///   existing resource.
/// - **Delete**: Lifecycle end. Removes the resource.
/// - **Action**: Extensibility. Executes a custom [`ActorService::Action`].
///
/// # Resource Interaction
/// This type is generic over `T: ActorService`. It uses the associated types
/// defined in the [`ActorService`] trait (like `Create`, `Update`, `Action`) to
/// ensure type safety for every operation. This guarantees that you can't send
/// a "User Create" payload to a "Product" actor.
#[derive(Debug)]
pub enum ResourceRequest<T: ActorService> {
    Create {
        params: T::Create,
        id: Option<T::Id>,
        respond_to: Response<T::Id>,
    },
    Get {
        id: T::Id,
        respond_to: Response<Option<T::Entity>>,
    },
    Update {
        id: T::Id,
        update: T::Update,
        respond_to: Response<T::Entity>,
    },
    #[allow(dead_code)]
    Delete {
        id: T::Id,
        respond_to: Response<()>,
    },
    Action {
        id: Option<T::Id>,
        action: T::Action,
        respond_to: Response<T::ActionResult>,
    },
    List {
        respond_to: Response<Vec<T::Entity>>,
    },
}
