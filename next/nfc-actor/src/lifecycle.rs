// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorLifecycle, DynContext};
use bytes::Bytes;
use tracing::info;

use crate::nfc_actor::NfcActor;

impl ActorLifecycle for NfcActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {
        info!("NfcActor started");
    }

    async fn on_stream(&mut self, id: Self::Id, message: Bytes, ctx: &mut DynContext<Self>) {
        if message.is_empty() {
            return;
        }
        self.handle_packet_stream(id, message, ctx).await;
    }

    async fn on_stream_closed(&mut self, id: Self::Id, ctx: &mut DynContext<Self>) {
        info!("Stream closed for NFC chip {}", id);
        use actor_framework::ActorService;
        let _ = self.handle_delete(id, ctx).await;
        ctx.abort(id);
    }
}

impl NfcActor {
    async fn handle_packet_stream(
        &mut self,
        id: netsim_model::ChipId,
        message: Bytes,
        _ctx: &mut DynContext<Self>,
    ) {
        // Stub implementation: just log packet reception.
        // In future, this will forward to Casimir.
        info!("NfcActor received {} bytes for chip {}", message.len(), id);
    }
}
