// Copyright 2025-2026 The Android Open Source Project

use crate::ap_actor::{ApActor, ApReq, ApResponse, ApState, WIFI_STREAM_ID};
use crate::error::ApError;
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;

#[async_trait]
impl ActorService for ApActor {
    type Id = u32;
    type Create = crate::ap_actor::ApConfig;
    type Update = crate::ap_actor::ApUpdate;
    type Action = ApReq;
    type ActionResult = ApResponse;

    type Error = ApError;
    type Entity = ApState;

    // ... (rest)
    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        config: Self::Create,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let id = self.next_ap_id;
        self.next_ap_id += 1;

        self.shared_keys.set_bssid(config.bssid);
        self.aps.insert(id, ApState { config, wpa: None });

        log::info!("Created AP with ID: {}", id);
        Ok(id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.aps.get(&id).cloned())
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        if let Some(ap) = self.aps.get_mut(&id) {
            if let Some(ssid) = update.ssid {
                ap.config.ssid = ssid;
            }
            // For now only SSID update supported
            Ok(ap.clone())
        } else {
            Err(ApError::ApNotFound(id))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        if self.aps.remove(&id).is_some() {
            log::info!("Deleted AP with ID: {}", id);
        } else {
            log::warn!("Attempted to delete non-existent AP with ID: {}", id);
        }
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.aps.values().cloned().collect())
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            ApReq::Register { stream, sink } => {
                if self.sink.is_some() {
                    panic!("ApActor: Register called more than once!");
                }
                log::info!("Registering AP singleton stream/sink");
                self.sink = Some(sink);

                let stream_id = WIFI_STREAM_ID;

                // Create a Send-capable Stream from the UnboundedReceiver
                let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(stream);
                let stream: std::pin::Pin<
                    Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>,
                > = Box::pin(stream);

                ctx.add_stream(stream_id, stream);

                ctx.set_interval(std::time::Duration::from_millis(100));
                Ok(ApResponse::Ok)
            }
        }
    }
}
