//! # ActorClient Trait
//!
//! Provides a common interface for resource‑specific clients, adding default `get` and `delete` methods built on top of a generic `ResourceClient`.
use crate::{ActorService, FrameworkError, ResourceClient};
use async_trait::async_trait;

/// Trait for resource-specific clients to inherit standard CRUD operations.
///
/// This trait reduces boilerplate by providing default implementations for
/// common operations like `get` and `delete`.
///
/// # Example
///
/// ```rust
/// use actor_framework::{ActorClient, ActorLifecycle, ActorService, BoxStream, Context, DynContext, FrameworkError, ResourceActor, ResourceClient};
/// use async_trait::async_trait;
///
/// // 1. Define Service
/// #[derive(Clone, Debug)]
/// struct User { id: u32 }
/// #[derive(Debug)] struct UserCreate;
/// #[derive(Debug)] struct UserUpdate;
/// #[derive(Debug)] enum UserAction {}
/// #[derive(Debug)] struct UserError(String);
///
/// // Error must implement Display + Error + From<String> + Send + Sync
/// impl std::fmt::Display for UserError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
/// impl std::error::Error for UserError {}
///
/// impl From<String> for UserError {
///     fn from(s: String) -> Self { UserError(s) }
/// }
///
/// #[async_trait]
/// impl ActorService for User {
///     type Id = u32;
///     type Create = UserCreate;
///     type Update = UserUpdate;
///     type Action = UserAction;
///     type ActionResult = ();
///     type Error = UserError;
///     type Entity = User;
///
///     async fn handle_create(
///         &mut self,
///         id: Option<u32>,
///         _: UserCreate,
///         _: &mut DynContext<Self::Id>,
///     ) -> Result<u32, Self::Error> {
///  self.id = id.unwrap_or(0); Ok(self.id) }
///     async fn handle_get(&self, _: u32, _: &mut DynContext<Self::Id>) -> Result<Option<Self::Entity>, Self::Error> { Ok(Some(self.clone())) }
///     async fn handle_update(&mut self, _: u32, _: UserUpdate, _: &mut DynContext<Self::Id>) -> Result<Self::Entity, Self::Error> { Ok(self.clone()) }
///     async fn handle_delete(&mut self, _: u32, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
///     async fn handle_action(&mut self, _id: Option<Self::Id>, _: UserAction, _: &mut DynContext<Self::Id>) -> Result<(), Self::Error> { Ok(()) }
///     async fn handle_list(&mut self, _: &mut DynContext<Self::Id>) -> Result<Vec<User>, Self::Error> { Ok(vec![self.clone()]) }
/// }
///
/// // 2. Define Client Wrapper
/// struct UserClient {
///     inner: ResourceClient<User>,
/// }
///
/// // 3. Implement ActorClient
/// #[async_trait]
/// impl ActorClient<User> for UserClient {
///     type Error = UserError;
///
///     fn inner(&self) -> &ResourceClient<User> {
///         &self.inner
///     }
///
///     fn map_error(e: FrameworkError) -> Self::Error {
///         UserError(e.to_string())
///     }
/// }
///
/// // 4. Usage
/// async fn usage(client: UserClient) {
///     // get() and delete() are provided automatically!
///     let _ = client.get(1).await;
///     let _ = client.delete(1).await;
/// }
/// ```
#[async_trait]
pub trait ActorClient<T: ActorService>: Send + Sync {
    /// The resource-specific error type.
    type Error: From<String> + Send + Sync;

    /// Access the inner generic ResourceClient.
    fn inner(&self) -> &ResourceClient<T>;

    /// Map framework errors to the specific resource error type.
    fn map_error(e: FrameworkError) -> Self::Error;

    /// Fetch a resource by ID.
    // #[tracing::instrument(skip(self))]
    async fn get(&self, id: T::Id) -> Result<Option<T::Entity>, Self::Error> {
        // tracing::debug!("Sending request");
        self.inner().get(id).await.map_err(Self::map_error)
    }

    /// Delete a resource by ID.
    // #[tracing::instrument(skip(self))]
    async fn delete(&self, id: T::Id) -> Result<(), Self::Error> {
        // tracing::debug!("Sending request");
        self.inner().delete(id).await.map_err(Self::map_error)
    }
}
