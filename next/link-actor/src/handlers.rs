// Copyright 2025 The Android Open Source Project

use crate::entity::{LinkContext, LinkEntity};
use crate::error::LinkError;
use actor_framework::Runtime;
use link_api::LinkAction;

pub fn handle_action(
    _entity: &mut LinkEntity,
    action: LinkAction,
    ctx: &mut LinkContext,
    _rt: &mut impl Runtime,
) -> Result<(), LinkError> {
    match action {
        LinkAction::NotifyChipAdded(chip_id, chip_kind) => {
            // TODO: Handle chip added (e.g. check for new links)
            // TODO: Send updates to any ChipActor with dest chip_id
            log::info!("NotifyChipAdded: {} {:?}", chip_id, chip_kind);
            ctx.chip_kind_map.insert(chip_id, chip_kind);
        }
        LinkAction::NotifyChipRemoved(chip_id) => {
            // TODO: Handle chip removed
            // TODO: Remove any LinkEntity with chip_id as source or dest
            log::info!("NotifyChipRemoved: {}", chip_id);
            ctx.chip_kind_map.remove(&chip_id);
        }
    }
    Ok(())
}
