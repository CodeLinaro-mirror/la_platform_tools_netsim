// Copyright 2025 The Android Open Source Project

use crate::error::LinkError;
use crate::link_actor::LinkActor;
use actor_framework::{ActorService, Context};
use async_trait::async_trait;
use link_api::LinkAction;

#[async_trait]
impl ActorService for LinkActor {
    type Id = link_api::LinkId;
    type Create = link_api::LinkCreate;
    type Update = ();
    type Action = LinkAction;
    type ActionResult = ();
    type Error = LinkError;
    type Entity = link_api::Link;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut impl Context,
    ) -> Result<Self::Id, Self::Error> {
        let id = id.unwrap_or_else(|| {
            let id = link_api::LinkId(self.next_id);
            self.next_id += 1;
            id
        });

        // Create Entity (Link)
        let mut link = link_api::Link {
            id,
            sender: params.sender,
            receiver: params.receiver,
            kind: netsim_model::chip::ChipKind::UNSPECIFIED,
            rssi: params.rssi,
        };

        // Validate chips exist and have matching kinds
        let sender_kind = self
            .chip_kind_map
            .get(&link.sender)
            .ok_or(LinkError::InvalidParam(format!("Sender chip {} not found", link.sender)))?;
        let receiver_kind = self
            .chip_kind_map
            .get(&link.receiver)
            .ok_or(LinkError::InvalidParam(format!("Receiver chip {} not found", link.receiver)))?;

        if sender_kind != receiver_kind {
            return Err(LinkError::InvalidParam(format!(
                "Chip kind mismatch: sender={:?}, receiver={:?}",
                sender_kind, receiver_kind
            )));
        }

        link.kind = *sender_kind;
        self.lookup.insert((link.sender, link.receiver), link.id);

        // TODO: Forward to radio client
        self.links.insert(id, link);
        Ok(id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut impl Context,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(actor_framework::utils::handle_get_default(&self.links, &id))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        _update: Self::Update,
        _ctx: &mut impl Context,
    ) -> Result<Self::Entity, Self::Error> {
        if let Some(link) = self.links.get(&id) {
            Ok(link.clone())
        } else {
            Err(LinkError::NotFound(id.to_string()))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        if let Some(link) = self.links.remove(&id) {
            self.lookup.remove(&(link.sender, link.receiver));
            // TODO: Forward delete to radio client
            Ok(())
        } else {
            Err(LinkError::NotFound(id.to_string()))
        }
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        _ctx: &mut impl Context,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            LinkAction::NotifyChipAdded(chip_id, chip_kind) => {
                log::info!("NotifyChipAdded: {} {:?}", chip_id, chip_kind);
                self.chip_kind_map.insert(chip_id, chip_kind);
            }
            LinkAction::NotifyChipRemoved(chip_id) => {
                log::info!("NotifyChipRemoved: {}", chip_id);
                self.chip_kind_map.remove(&chip_id);
            }
        }
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut impl Context,
    ) -> Result<Vec<link_api::Link>, Self::Error> {
        Ok(actor_framework::utils::handle_list_map(&self.links, |e| e.clone()))
    }
}
