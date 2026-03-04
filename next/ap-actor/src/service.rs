// Copyright 2025-2026 The Android Open Source Project

use std::sync::Arc;

use actor_framework::{ActorService, DynContext};

use crate::{
    ap_actor::{ApActor, ApReq, ApResponse, ApState, WIFI_STREAM_ID},
    error::ApError,
};

// 1024 microseconds per Time Unit (TU)
const TU_INTERVAL_US: u128 = 1024;

impl ActorService for ApActor {
    type Id = crate::ap_actor::ApId;
    type Create = netsim_model::chip::ApCreate;
    type Update = crate::ap_actor::ApActorUpdate;
    type Action = ApReq;
    type ActionResult = ApResponse;

    type Error = ApError;
    type Entity = ApState;
    type TypedStream = ();

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        params: Self::Create,
        _: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let config = crate::ap_actor::ApConfig::try_from(params).map_err(ApError::Internal)?;

        let id_val = self.next_id;
        let ap_id = crate::ap_actor::ApId(id_val);

        if self.aps.contains_key(&ap_id) {
            panic!("AP with ID {} already exists", id_val);
        }

        self.next_id += 1;
        self.shared_keys.set_bssid(config.bssid);
        self.aps.insert(ap_id, ApState::new(ap_id, config));

        log::info!("Created AP with ID: {}", id_val);
        Ok(ap_id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.aps.get(&id).cloned())
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        patch: Self::Update,
        _: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        let ap_state = self.aps.get_mut(&id).ok_or(ApError::ApNotFound(id.0))?;

        if let Some(pos) = patch.position {
            ap_state.config.position = pos;
        }

        let ap_u = patch.variant;
        if let Some(ssid) = ap_u.ssid {
            ap_state.config.ssid = ssid;
        }
        if let Some(channel) = ap_u.channel {
            ap_state.config.channel = channel;
        }
        if !ap_u.force_disconnect.is_empty() {
            log::warn!("Ignored force_disconnect in Update. Use Disconnect RPC instead.");
        }

        if let Some(enabled) = patch.enabled {
            ap_state.enabled = enabled;
        }

        Ok(ap_state.clone())
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _: &mut DynContext<Self>,
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
        _: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.aps.values().cloned().collect())
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match action {
            ApReq::Register { stream, sink, shared_keys, beacon_interval } => {
                if self.sink.is_some() {
                    panic!("ApActor: Register called more than once!");
                }
                log::info!(
                    "Registering AP singleton stream/sink with interval: {:?}",
                    beacon_interval
                );
                self.sink = Some(sink);
                if !Arc::ptr_eq(&self.shared_keys, &shared_keys) {
                    // Preserve BSSID if the new store doesn't have one (Contextual Strangler fix)
                    if let Some(current_bssid) = self.shared_keys.get_bssid() {
                        if shared_keys.get_bssid().is_none() {
                            shared_keys.set_bssid(current_bssid);
                            log::info!(
                                "ApActor: Preserved BSSID {:?} in shared_keys",
                                current_bssid
                            );
                        }
                    }
                    self.shared_keys = shared_keys;
                }

                let tus = (beacon_interval.as_micros() / TU_INTERVAL_US) as u16;
                self.beacon_interval = Some(tus);

                let stream_id = WIFI_STREAM_ID;
                ctx.add_stream(crate::ap_actor::ApId(stream_id), stream);

                ctx.set_interval(beacon_interval);
                Ok(ApResponse::Ok)
            }
            ApReq::Disconnect { id, mac } => {
                let ap_state = self.aps.get_mut(&id).ok_or(ApError::ApNotFound(id.0))?;

                if !ap_state.associations.remove(&mac) {
                    log::warn!("Requested disconnect for unknown MAC: {}", mac);
                    return Ok(ApResponse::Ok);
                }

                // Also clear sessions if any
                if let Some(wpa) = &mut ap_state.wpa {
                    wpa.remove_session(&mac);
                }
                ap_state.sae_sessions.remove(&mac);
                ap_state.eap_sessions.remove(&mac);

                // Send Deauth Frame
                if let Some(sink) = &self.sink {
                    // Reason Code 3 (Deauthenticated because sending STA is leaving (or has left)
                    // IBSS or ESS)
                    let frame = self.manager.build_deauth_frame(ap_state, mac, 3);
                    if let Err(e) = sink.send(bytes::Bytes::from(frame)) {
                        log::error!("Failed to send Deauth frame: {}", e);
                    }
                }
                Ok(ApResponse::Ok)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actor_framework::Context;
    use netsim_model::ap::ApCreate;

    use super::*;
    use crate::ap_actor::{ApActor, ApConfig, ApId};

    fn create_test_actor(start_id: u32) -> ApActor {
        let shared_keys = Arc::new(crate::shared::SharedKeyStore::new());
        let mut actor = ApActor::new(shared_keys);
        actor.next_id = start_id;
        actor
    }

    fn create_ap_params() -> ApCreate {
        ApCreate { ssid: "test-ap".into(), bssid: "02:00:00:00:00:00".into(), ..Default::default() }
    }

    // --- Test Context Implementation ---

    use std::time::Duration;

    use actor_framework::{BoxStream, BoxTypedStream, TimerKey};
    use futures::future::BoxFuture;

    struct TestContext {
        queue: tokio_util::time::DelayQueue<()>,
    }

    impl TestContext {
        fn new() -> Self {
            Self { queue: tokio_util::time::DelayQueue::new() }
        }
    }

    impl Context<ApActor> for TestContext {
        fn set_interval(&mut self, _duration: Duration) {}
        fn add_stream(&mut self, _id: ApId, _stream: BoxStream) {}
        fn remove_stream(&mut self, _id: ApId) {}
        fn add_typed_stream(&mut self, _id: usize, _stream: BoxTypedStream<()>) {}
        fn remove_typed_stream(&mut self, _id: usize) {}
        fn spawn(&mut self, _id: ApId, _task: BoxFuture<'static, ApId>) {}
        fn abort(&mut self, _id: ApId) {}
        fn shutdown(&mut self) {}
        fn run_later(
            &mut self,
            duration: Duration,
            _f: Box<dyn FnOnce(&mut ApActor, &mut dyn Context<ApActor>) + Send>,
        ) -> TimerKey {
            self.queue.insert((), duration)
        }
        fn cancel_timer(&mut self, key: TimerKey) {
            self.queue.remove(&key);
        }
    }

    #[tokio::test]
    async fn test_create_ignores_requested_id() {
        let mut actor = create_test_actor(100);
        let mut ctx = TestContext::new();

        // Request ID 0 (passed as Option<ApId>)
        let id1 = actor
            .handle_create(Some(ApId(0)), create_ap_params(), &mut ctx)
            .await
            .expect("Failed to create AP");
        assert_eq!(id1.0, 100);

        // Request specific ID 999 - should be ignored and use next_id (101)
        let id2 = actor
            .handle_create(Some(ApId(999)), create_ap_params(), &mut ctx)
            .await
            .expect("Failed to create AP");
        assert_eq!(id2.0, 101);
    }

    #[tokio::test]
    #[should_panic(expected = "AP with ID 10 already exists")]
    async fn test_create_fails_on_existing_id() {
        let mut actor = create_test_actor(10);
        let mut ctx = TestContext::new();

        // Manually insert an AP with ID 10
        let config = ApConfig::default();
        actor.aps.insert(ApId(10), ApState::new(ApId(10), config));

        // Next ID is 10, but it exists. Should fail.
        let _ = actor.handle_create(None, create_ap_params(), &mut ctx).await;
    }
}
