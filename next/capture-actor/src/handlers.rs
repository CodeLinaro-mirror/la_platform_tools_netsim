//! This module contains handlers for the CaptureActor.
//!
//! # Why manual action handling?
//! While the actor framework provides standard CRUD operations, we implement custom
//! handlers for `Get` and `List` here because we need to return enriched data
//! (specifically, packet capture statistics from the `CaptureWriter`) which is not
//! part of the standard `CaptureEntity` state.

use crate::actions::{CaptureAction, CaptureActionResult, CaptureInfo};
use crate::context::CaptureContext;
use crate::entity::CaptureEntity;
use crate::error::CaptureError;
use actor_framework::ActorEntity;
use netsim_model::chip::ChipId;
use std::time::SystemTime;

pub async fn on_create(
    entity: &mut CaptureEntity,
    ctx: &mut CaptureContext,
) -> Result<(), CaptureError> {
    ctx.flags.insert(entity.chip_id, entity.enabled_flag.clone());
    Ok(())
}

pub async fn handle_action(
    entity: &mut CaptureEntity,
    action: CaptureAction,
    ctx: &mut CaptureContext,
) -> Result<CaptureActionResult, CaptureError> {
    match action {
        CaptureAction::CapturePacket { chip_id, direction, bytes } => {
            if !entity.enabled || entity.chip_id != chip_id {
                return Ok(CaptureActionResult::Success);
            }
            if let Some(writer) = ctx.writers.get_mut(&chip_id) {
                writer.write_packet(SystemTime::now(), direction, &bytes)?;
            }
            Ok(CaptureActionResult::Success)
        }
        CaptureAction::Get { chip_id } => {
            if entity.chip_id != chip_id {
                return Ok(CaptureActionResult::Get(None));
            }
            let (records, bytes) =
                ctx.writers.get(&entity.chip_id).map(|w| w.get_stats()).unwrap_or((0, 0));
            Ok(CaptureActionResult::Get(Some(CaptureInfo {
                chip_id: entity.chip_id,
                chip_kind: entity.chip_kind,
                device_name: entity.device_name.clone(),
                enabled: entity.enabled,
                records_written: records,
                bytes_written: bytes,
            })))
        }
        CaptureAction::Patch { chip_id, enabled } => {
            if entity.chip_id != chip_id {
                return Ok(CaptureActionResult::Success);
            }
            entity.on_update(enabled, ctx).await?;
            Ok(CaptureActionResult::Success)
        }
        CaptureAction::Create { chip_id: _, chip_kind: _, device_name: _ } => {
            // This is usually handled by the actor framework's create,
            // but if called as an action on an existing entity, it's probably an error or we update.
            // For now, assume it's handled by the framework.
            Ok(CaptureActionResult::Success)
        }
        CaptureAction::Delete { chip_id } => {
            if entity.chip_id != chip_id {
                return Ok(CaptureActionResult::Success);
            }
            entity.on_delete(ctx).await?;
            // Note: The entity itself is not deleted from the actor here,
            // the caller should call delete on the actor.
            Ok(CaptureActionResult::Success)
        }
        CaptureAction::SetDefaultCapture { enabled } => {
            ctx.default_capture_enabled = enabled;
            Ok(CaptureActionResult::Success)
        }
        CaptureAction::SetCaptureDirectory { path } => {
            ctx.capture_dir = Some(path);
            Ok(CaptureActionResult::Success)
        }
    }
}

pub fn on_list(
    entities: &std::collections::HashMap<ChipId, CaptureEntity>,
    ctx: &mut CaptureContext,
) -> Vec<CaptureInfo> {
    entities
        .values()
        .map(|e| {
            let (records_written, bytes_written) =
                ctx.writers.get(&e.chip_id).map(|w| w.get_stats()).unwrap_or((0, 0));
            CaptureInfo {
                chip_id: e.chip_id,
                chip_kind: e.chip_kind,
                device_name: e.device_name.clone(),
                enabled: e.enabled,
                records_written,
                bytes_written,
            }
        })
        .collect()
}
