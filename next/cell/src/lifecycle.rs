// Copyright 2025 The Android Open Source Project

use actor_framework::{ActorLifecycle, ActorService, DynContext};
use bytes::Bytes;
use modem_rs::HostEvent;

use crate::cell_actor::{CellActor, CLIENT_EVENT_ID};

impl ActorLifecycle for CellActor {
    async fn on_start(&mut self, ctx: &mut DynContext<Self>) {
        if let Some(rx) = self.event_receiver.take() {
            let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
            ctx.add_stream(CLIENT_EVENT_ID, Box::pin(stream));
        }
    }

    async fn on_stream(&mut self, id: Self::Id, message: Bytes, ctx: &mut DynContext<Self>) {
        if message.is_empty() {
            return;
        }

        if id == CLIENT_EVENT_ID {
            self.handle_host_event(message, ctx).await;
        } else {
            self.handle_packet_stream(id, message, ctx).await;
        }
    }

    async fn on_stream_closed(&mut self, id: Self::Id, ctx: &mut DynContext<Self>) {
        log::info!("Stream closed for chip {}", id);
        use actor_framework::ActorService;
        let _ = self.handle_delete(id, ctx).await;
        ctx.abort(id);
    }
}

impl CellActor {
    async fn handle_packet_stream(
        &mut self,
        id: netsim_model::chip::ChipId,
        message: Bytes,
        _ctx: &mut DynContext<Self>,
    ) {
        if let Err(e) = self.controller.send_data(id.0, &message) {
            log::error!("Failed to send data to controller for chip {}: {:?}", id, e);
        }
    }

    async fn handle_host_event(&mut self, message: Bytes, ctx: &mut DynContext<Self>) {
        let event = HostEvent::parse(message.as_ref());

        match event {
            HostEvent::SinkError(id) => {
                let chip_id = netsim_model::chip::ChipId(id);
                log::warn!("Sink error for chip {}, deleting.", chip_id);
                let _ = self.handle_get(chip_id, ctx).await;
                ctx.remove_stream(chip_id);
                ctx.abort(chip_id);
            }

            HostEvent::TimerRequest { chip_id: id, duration } => {
                // Schedule the internal timer callback
                ctx.run_later(
                    duration,
                    Box::new(move |actor, _ctx| {
                        if let Err(e) = actor.controller.on_timer(id) {
                            log::error!("Timer error for chip {}: {:?}", id, e);
                        }
                    }),
                );
            }
        }
    }
}
