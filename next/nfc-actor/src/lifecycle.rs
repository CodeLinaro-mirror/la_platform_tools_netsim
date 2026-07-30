// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use actor_framework::{ActorLifecycle, DynContext};
use bytes::Bytes;
use casimir::packets::nci::{ControlPacket, ControlPacketChild, CorePacketChild, RfPacketChild};
use pdl_runtime::Packet;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

use crate::{nfc_actor::NfcActor, stats::NfcApi};

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
        if message.is_empty() {
            return;
        }
        let mt = message[0] >> 5;
        if mt == 0 {
            // Data Packet
            if message.len() >= 3 {
                self.nfc_stats.incr(NfcApi::DataSend);
            } else {
                tracing::warn!(
                    "Failed to decode DataPacket (invalid length), bytes: {:?}",
                    message
                );
            }
        } else {
            // Control Packet
            if let Ok(packet) = ControlPacket::decode_full(&message) {
                match packet.specialize() {
                    Ok(ControlPacketChild::CorePacket(core_pkt)) => match core_pkt.specialize() {
                        Ok(CorePacketChild::CoreResetCommand(_)) => {
                            self.nfc_stats.incr(NfcApi::CoreReset);
                        }
                        Ok(CorePacketChild::CoreInitCommand(_)) => {
                            self.nfc_stats.incr(NfcApi::CoreInit);
                        }
                        _ => {}
                    },
                    Ok(ControlPacketChild::RfPacket(rf_pkt)) => match rf_pkt.specialize() {
                        Ok(RfPacketChild::RfDiscoverCommand(_)) => {
                            self.nfc_stats.incr(NfcApi::RfDiscover);
                        }
                        Ok(RfPacketChild::RfDiscoverSelectCommand(_)) => {
                            self.nfc_stats.incr(NfcApi::RfDiscoverSelect);
                        }
                        Ok(RfPacketChild::RfDeactivateCommand(_)) => {
                            self.nfc_stats.incr(NfcApi::RfDeactivate);
                        }
                        Ok(RfPacketChild::RfSetListenModeRoutingCommand(_)) => {
                            self.nfc_stats.incr(NfcApi::RfSetListenModeRouting);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            } else {
                tracing::warn!("Failed to decode ControlPacket, bytes: {:?}", message);
            }
        }

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
