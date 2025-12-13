use crate::actor::DeviceActor;
use crate::error::DeviceError;
use crate::DeviceEntity;
use actor_framework::{ActorService, Context, ResourceActor, ResourceClient};
use async_trait::async_trait;
use device_api::api::{DeviceCreate, DeviceUpdate, ListDeviceResponse};
use device_api::DeviceId;
use device_api::{DeviceAction, DeviceActionResult};

#[async_trait]
impl ActorService for DeviceEntity {
    type Id = DeviceId;
    type Create = DeviceCreate;
    type Update = DeviceUpdate;
    type Action = DeviceAction;
    type ActionResult = DeviceActionResult;
    type Context = DeviceActor;
    type Error = DeviceError;
    type ListResponse = ListDeviceResponse;

    /// Creates a new Device from creation parameters.
    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error> {
        Ok(DeviceEntity {
            device: device_api::Device {
                id: id.0,
                name: params.device_config.name.clone(),
                visible: params.device_config.visible,
                position: params.device_config.position.clone(),
                orientation: params.device_config.orientation.clone(),
                chips: vec![], // Will be filled in on_create
            },
            create_params: Some(params),
        })
    }

    /// Called after the entity is created.
    /// This is where we handle side effects of creation, like creating associated chips.
    async fn on_create(
        &mut self,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        crate::handlers::on_create(self, actor, ctx).await
    }

    /// Handles custom actions for the Device actor.
    async fn handle_action(
        &mut self,
        action: Self::Action,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<Self::ActionResult, Self::Error> {
        crate::handlers::handle_action(self, action, actor, ctx).await
    }

    /// Called when the entity is updated.
    /// Propagates relevant updates (like position/orientation) to associated chips.
    async fn on_update(
        &mut self,
        update: Self::Update,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        crate::handlers::on_update(self, update, actor, ctx).await
    }

    /// Called when the entity is deleted.
    /// Ensures all associated chips are also deleted.
    async fn on_delete(
        &self,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        crate::handlers::on_delete(self, actor, ctx).await
    }

    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        _context: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Self::ListResponse {
        crate::handlers::on_list(entities)
    }
}
