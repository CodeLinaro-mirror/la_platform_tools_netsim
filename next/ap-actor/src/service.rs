// Copyright 2025-2026 The Android Open Source Project

use crate::ap_actor::{ApActor, ApReq, ApResponse, ApState, WIFI_STREAM_ID};
use crate::error::ApError;
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;

use netsim_model::chip::{
    ApChip, Chip, ChipCreate, ChipId, ChipKind, ChipUpdate, ChipVariant, ChipVariantUpdate,
    NetworkParams,
};

use netsim_model::device::DeviceId;

// 1024 microseconds per Time Unit (TU)
const TU_INTERVAL_US: u128 = 1024;

#[async_trait]
impl ActorService for ApActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = ApReq;
    type ActionResult = ApResponse;

    type Error = ApError;
    type Entity = Chip;

    async fn handle_create(
        &mut self,
        _id: Option<Self::Id>,
        params: Self::Create,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let id_val = params.id.0;
        if self.aps.contains_key(&id_val) {
            return Err(ApError::Internal(format!("AP with ID {} already exists", id_val)));
        }

        let config = if let NetworkParams::Ap(ap_create) = params.config.network_params {
            crate::ap_actor::ApConfig::try_from(ap_create).map_err(|e| ApError::Internal(e))?
        } else {
            return Err(ApError::Internal("Invalid NetworkParams for AP".into()));
        };

        self.shared_keys.set_bssid(config.bssid);
        self.aps.insert(id_val, ApState::new(config));

        log::info!("Created AP with ID: {}", id_val);
        Ok(ChipId(id_val))
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        if let Some(state) = self.aps.get(&id.0) {
            Ok(Some(ap_state_to_chip(id.0, state)))
        } else {
            Ok(None)
        }
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        patch: Self::Update,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        let ap_state = self.aps.get_mut(&id.0).ok_or(ApError::ApNotFound(id.0))?;

        if let Some(pos) = patch.position {
            ap_state.config.position = pos;
        }

        if let Some(ChipVariantUpdate::Ap(ap_u)) = patch.variant {
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
                    // Reason Code 3 (Deauthenticated because sending STA is leaving (or has left) IBSS or ESS)
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

        Ok(ap_state_to_chip(id.0, ap_state))
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        if self.aps.remove(&id.0).is_some() {
            log::info!("Deleted AP with ID: {}", id.0);
        } else {
            log::warn!("Attempted to delete non-existent AP with ID: {}", id.0);
        }
        Ok(())
    }

    async fn handle_list(
        &mut self,
        _: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(self.aps.iter().map(|(id, state)| ap_state_to_chip(*id, state)).collect())
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        action: Self::Action,
        ctx: &mut DynContext<Self::Id>,
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
                self.shared_keys = shared_keys;

                let tus = (beacon_interval.as_micros() / TU_INTERVAL_US) as u16;
                self.beacon_interval = Some(tus);

                let stream_id = ChipId(WIFI_STREAM_ID);
                ctx.add_stream(stream_id, stream);

                ctx.set_interval(beacon_interval);
                Ok(ApResponse::Ok)
            }
        }
    }
}

fn ap_state_to_chip(id: u32, state: &ApState) -> Chip {
    Chip {
        id,
        kind: ChipKind::AP,
        name: Some(state.config.ssid.clone()),
        manufacturer: Some("Netsim".into()),
        product_name: Some("AccessPoint".into()),
        position: state.config.position.clone(),
        orientation: Default::default(),
        device_id: DeviceId(0),
        variant: Some(ChipVariant::Ap(ApChip {
            config: state.config.clone().into(),
            associations: state.associations.iter().map(ToString::to_string).collect(),
        })),
        links: Vec::new(),
        enabled: true,
    }
}
