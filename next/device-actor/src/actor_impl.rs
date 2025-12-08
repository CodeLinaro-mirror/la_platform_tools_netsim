use crate::context::DeviceContext;
use crate::entity::DeviceEntity;
use crate::error::DeviceError;
use actor_framework::ActorEntity;
use async_trait::async_trait;
use device_api::api::{DeviceCreate, DeviceUpdate, ListDeviceResponse};
use device_api::DeviceId;
use device_api::{DeviceAction, DeviceActionResult};

#[async_trait]
impl ActorEntity for DeviceEntity {
    type Id = DeviceId;
    type Create = DeviceCreate;
    type Update = DeviceUpdate;
    type Action = DeviceAction;
    type ActionResult = DeviceActionResult;
    type Context = DeviceContext;
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
    async fn on_create(&mut self, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        crate::handlers::on_create(self, ctx).await
    }

    /// Handles custom actions for the Device actor.
    async fn handle_action(
        &mut self,
        action: Self::Action,
        ctx: &mut Self::Context,
    ) -> Result<Self::ActionResult, Self::Error> {
        crate::handlers::handle_action(self, action, ctx).await
    }

    /// Called when the entity is updated.
    /// Propagates relevant updates (like position/orientation) to associated chips.
    async fn on_update(
        &mut self,
        update: Self::Update,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        crate::handlers::on_update(self, update, ctx).await
    }

    /// Called when the entity is deleted.
    /// Ensures all associated chips are also deleted.
    async fn on_delete(&self, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        crate::handlers::on_delete(self, ctx).await
    }

    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        _ctx: &mut Self::Context,
    ) -> Self::ListResponse {
        let devices = entities.values().map(|e| e.device.clone()).collect();
        ListDeviceResponse { devices }
    }
}
