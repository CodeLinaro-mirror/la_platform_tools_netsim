// Copyright 2025 The Android Open Source Project

use crate::entity::{LinkContext, LinkEntity};
use crate::error::LinkError;
use crate::handlers;
use actor_framework::{ActorEntity, Runtime};
use async_trait::async_trait;
use link_api::LinkAction;

#[async_trait]
impl ActorEntity for LinkEntity {
    type Id = link_api::LinkId;
    type Create = link_api::LinkCreate;
    type Update = ();
    type Action = LinkAction;
    type ActionResult = ();
    type Context = LinkContext;
    type Error = LinkError;
    type ListResponse = Vec<link_api::Link>;

    fn from_create_params(id: Self::Id, params: Self::Create) -> Result<Self, Self::Error> {
        Ok(LinkEntity::new(link_api::Link {
            id,
            sender: params.sender,
            receiver: params.receiver,
            kind: netsim_model::chip::ChipKind::UNSPECIFIED,
            rssi: params.rssi,
        }))
    }

    async fn on_create(
        &mut self,
        ctx: &mut Self::Context,
        _rt: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        // Validate chips exist and have matching kinds
        let sender_kind = ctx.chip_kind_map.get(&self.link.sender).ok_or(
            LinkError::InvalidParam(format!("Sender chip {} not found", self.link.sender)),
        )?;
        let receiver_kind = ctx.chip_kind_map.get(&self.link.receiver).ok_or(
            LinkError::InvalidParam(format!("Receiver chip {} not found", self.link.receiver)),
        )?;

        if sender_kind != receiver_kind {
            return Err(LinkError::InvalidParam(format!(
                "Chip kind mismatch: sender={:?}, receiver={:?}",
                sender_kind, receiver_kind
            )));
        }

        // Set the kind
        self.link.kind = *sender_kind;

        ctx.lookup.insert((self.link.sender, self.link.receiver), self.link.id);

        // TODO: Forward to radio client
        Ok(())
    }

    async fn on_update(
        &mut self,
        _update: Self::Update,
        _ctx: &mut Self::Context,
        _rt: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn on_delete(
        &self,
        ctx: &mut Self::Context,
        _rt: &mut impl Runtime,
    ) -> Result<(), Self::Error> {
        ctx.lookup.remove(&(self.link.sender, self.link.receiver));
        // TODO: Forward delete to radio client
        Ok(())
    }

    async fn handle_action(
        &mut self,
        action: Self::Action,
        ctx: &mut Self::Context,
        rt: &mut impl Runtime,
    ) -> Result<Self::ActionResult, Self::Error> {
        handlers::handle_action(self, action, ctx, rt)
    }

    fn on_list(
        entities: &std::collections::HashMap<Self::Id, Self>,
        _ctx: &mut Self::Context,
        _rt: &mut impl Runtime,
    ) -> Self::ListResponse {
        entities.values().map(|e| e.link.clone()).collect()
    }
}
