// Copyright 2025 The Android Open Source Project

use crate::service::LinkActor;
use crate::service::LinkEntity;
use crate::LinkError;
use actor_framework::Context;
use link_api::LinkAction;

pub fn handle_action(
    _entity: &mut LinkEntity,
    action: LinkAction,
    actor: &mut LinkActor,
    _ctx: &mut impl Context,
) -> Result<(), LinkError> {
    match action {
        LinkAction::NotifyChipAdded(chip_id, chip_kind) => {
            // TODO: Handle chip added (e.g. check for new links)
            // TODO: Send updates to any ChipActor with dest chip_id
            log::info!("NotifyChipAdded: {} {:?}", chip_id, chip_kind);
            actor.chip_kind_map.insert(chip_id, chip_kind);
        }
        LinkAction::NotifyChipRemoved(chip_id) => {
            // TODO: Handle chip removed
            // TODO: Remove any LinkEntity with chip_id as source or dest
            log::info!("NotifyChipRemoved: {}", chip_id);
            actor.chip_kind_map.remove(&chip_id);
        }
    }
    Ok(())
}
