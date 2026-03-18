// Copyright 2025-2026 The Android Open Source Project

use actor_framework::{ActorService, DynContext};

use crate::{
    ap_actor::{ApActor, ApId, ApReq, ApResponse, ApState, WIFI_STREAM_ID},
    error::ApError,
};

// 1024 microseconds per Time Unit (TU)
const TU_INTERVAL_US: u128 = 1024;

impl ActorService for ApActor {
    type Id = ApId;
    type Create = crate::ap_actor::ApConfig;
    type Update = crate::ap_actor::ApActorUpdate;
    type Action = ApReq;
    type ActionResult = ApResponse;

    type Error = ApError;
    type Entity = ApState;
    type TypedStream = ();

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let id_val = match id {
            Some(i) => {
                if i.0 >= self.next_ap_id {
                    self.next_ap_id = i.0 + 1;
                }
                i
            }
            None => {
                let i = self.next_ap_id;
                self.next_ap_id += 1;
                ApId(i)
            }
        };

        if self.aps.contains_key(&id_val) {
            return Err(ApError::ApAlreadyExists(id_val.0));
        }

        let config = params;

        self.shared_keys.set_bssid(config.bssid);
        self.aps.insert(id_val, ApState::new(id_val, config));

        log::info!("Created AP with ID: {}", id_val);
        Ok(id_val)
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

        if let Some(ap_u) = patch.ap_update {
            if let Some(ssid) = ap_u.ssid {
                ap_state.config.ssid = ssid;
            }
            if let Some(channel) = ap_u.channel {
                ap_state.config.channel = channel;
            }
            for mac_str in ap_u.force_disconnect {
                // Find MAC for this string
                let mac = match mac_str.parse::<netsim_packets::ethernet::MacAddr>() {
                    Ok(m) => m,
                    Err(_) => {
                        log::warn!("Invalid MAC address in force_disconnect: {}", mac_str);
                        continue;
                    }
                };

                if !ap_state.associations.remove(&mac) {
                    log::warn!("Requested force disconnect for unknown MAC: {}", mac);
                    continue;
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
            }
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
        id: Option<Self::Id>,
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
                // Preserve BSSID if the new store doesn't have one (Contextual Strangler fix)
                if let Some(current_bssid) = self.shared_keys.get_bssid() {
                    if shared_keys.get_bssid().is_none() {
                        shared_keys.set_bssid(current_bssid);
                        log::info!("ApActor: Preserved BSSID {:?} in shared_keys", current_bssid);
                    }
                }
                self.shared_keys = shared_keys;

                let tus = (beacon_interval.as_micros() / TU_INTERVAL_US) as u16;
                self.beacon_interval = Some(tus);

                let stream_id = ApId(WIFI_STREAM_ID);
                ctx.add_stream(stream_id, stream);

                ctx.set_interval(beacon_interval);
                Ok(ApResponse::Ok)
            }
            ApReq::Disconnect { mac } => {
                let id = id.expect("ApActor: Disconnect requires an ID");
                let ap_state = self.aps.get_mut(&id).ok_or(ApError::ApNotFound(id.0))?;

                if ap_state.associations.remove(&mac) {
                    log::info!("ApActor: Force disconnecting MAC {} from AP {}", mac, id);

                    // Clear sessions if any
                    if let Some(wpa) = &mut ap_state.wpa {
                        wpa.remove_session(&mac);
                    }
                    ap_state.sae_sessions.remove(&mac);
                    ap_state.eap_sessions.remove(&mac);

                    // Send Deauth Frame
                    if let Some(sink) = &self.sink {
                        let frame = self.manager.build_deauth_frame(ap_state, mac, 3);
                        if let Err(e) = sink.send(bytes::Bytes::from(frame)) {
                            log::error!("Failed to send Deauth frame: {}", e);
                        }
                    }
                } else {
                    log::warn!(
                        "ApActor: Requested disconnect for unknown MAC {} on AP {}",
                        mac,
                        id
                    );
                }
                Ok(ApResponse::Ok)
            }
        }
    }
}
