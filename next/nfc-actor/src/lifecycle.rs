// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorLifecycle, DynContext};
use bytes::Bytes;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

use crate::nfc_actor::NfcActor;

impl ActorLifecycle for NfcActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {
        info!("NfcActor started");
        self.start_casimir();
    }

    async fn on_shutdown(&mut self) {
        if let Some(task) = self.scene_task.take() {
            info!("Shutting down Casimir scene task");
            task.abort();
        }
    }

    async fn on_stream(&mut self, id: Self::Id, message: Bytes, _ctx: &mut DynContext<Self>) {
        if let Some(state) = self.active_chips.get_mut(&id)
            && let Err(e) = state.nfc_writer.write_all(&message).await
        {
            error!("Failed to write guest packet to Casimir: {:?}", e);
        }
    }

    async fn on_stream_closed(&mut self, id: Self::Id, ctx: &mut DynContext<Self>) {
        info!("Stream closed for NFC chip {}", id);
        use actor_framework::ActorService;
        let _ = self.handle_delete(id, ctx).await;
    }

    async fn on_task_closed(&mut self, id: Self::Id, ctx: &mut DynContext<Self>) {
        info!("Task closed for NFC chip {}", id);
        use actor_framework::ActorService;
        let _ = self.handle_delete(id, ctx).await;
    }
}
