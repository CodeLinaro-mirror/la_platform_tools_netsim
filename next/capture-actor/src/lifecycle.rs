// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use actor_framework::{ActorLifecycle, DynContext};
use netsim_model::ChipId;
use tracing::{debug, error};

use crate::capture_actor::CaptureActor;

/// Periodically flush writers, ensuring packet captures are _roughly live_
/// and any truncation from a crash is minimal.
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

impl ActorLifecycle for CaptureActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        ctx.set_interval(FLUSH_INTERVAL);
    }

    async fn on_tick(&mut self, _ctx: &mut DynContext<Self>) {
        self.flush_writers().await;
    }

    async fn on_typed_stream(
        &mut self,
        id: usize,
        msg: Self::TypedStream,
        _ctx: &mut DynContext<Self>,
    ) {
        let chip_id = ChipId(id as u32);
        let (timestamp, direction, bytes) = msg;
        let Some(entity) = self.entities.get_mut(&chip_id) else {
            return;
        };

        if !entity.info.enabled {
            return;
        }

        if let Some(writer) = self.writers.get_mut(&chip_id) {
            if let Err(err) = writer.write_packet(timestamp, direction, &bytes).await {
                if !entity.has_warned_on_write {
                    entity.has_warned_on_write = true;
                    error!(
                        "Packet capture write failed for chip {chip_id}: {err}. Further errors for this chip will be suppressed."
                    );
                }
            }
        }
    }

    async fn on_typed_stream_closed(&mut self, id: usize, _ctx: &mut DynContext<Self>) {
        debug!("Typed stream closed for capture: {}", id);
    }

    async fn on_shutdown(&mut self) {
        self.flush_writers().await;
    }
}
