// Copyright 2025 The Android Open Source Project

use crate::error::WifiError;
use crate::wifi_actor::WifiActor;
use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;

use netsim_model::chip::ChipId;
/// ID for the AP infrastructure stream
pub const AP_ID: ChipId = ChipId(u32::MAX);
/// ID for the Slirp infrastructure stream
pub const SLIRP_ID: ChipId = ChipId(u32::MAX - 1);

#[async_trait]
impl ActorLifecycle<ChipId> for WifiActor {
    type Error = WifiError;

    async fn on_start(&mut self, ctx: &mut DynContext<ChipId>) {
        log::info!("WifiActor started");

        if let Some(ap_client) = &self.ap_client {
            // Uplink: Wifi -> AP
            // We hold the tx (to write to AP), AP gets the rx (to read from us)
            let (ap_uplink_tx, ap_uplink_rx) = create_channel_stream();

            // Downlink: AP -> Wifi
            // AP gets the tx (to write to us), We hold the rx (to read from AP)
            let (ap_downlink_tx, ap_downlink_rx) = create_channel_stream();

            // Wire AP Downlink to Context
            ctx.add_stream(AP_ID, ap_downlink_rx);

            // Default beacon interval for now (100ms)
            if let Err(e) = ap_client
                .register(
                    ap_uplink_rx,
                    ap_downlink_tx,
                    self.shared_keys.clone(),
                    std::time::Duration::from_millis(100),
                )
                .await
            {
                log::error!("Failed to register with AP client: {}", e);
            }
            self.to_ap = Some(ap_uplink_tx);
        }

        // Register Slirp Stream & Sink
        if let Some(client) = &self.slirp_client {
            // Uplink: Wifi -> Slirp
            // We hold the tx, Slirp gets the rx
            let (slirp_uplink_tx, slirp_uplink_rx) = create_channel_stream();

            // Downlink: Slirp -> Wifi
            // Slirp gets the tx, We hold the rx
            let (slirp_downlink_tx, slirp_downlink_rx) = create_channel_stream();

            // Wire Slirp Downlink to Context
            ctx.add_stream(SLIRP_ID, slirp_downlink_rx);

            // Register with SlirpActor
            if let Err(e) = client.register(slirp_uplink_rx, slirp_downlink_tx).await {
                log::error!("Failed to register slirp client: {}", e);
            }
            self.to_slirp = Some(slirp_uplink_tx);
        }

        // Create Default AndroidAp
        if self.create_default_ap {
            if let Some(ap_client) = &self.ap_client {
                log::info!("Creating default AndroidAp");
                let config = ap_actor::ApConfig {
                    ssid: "AndroidWifi".to_string(), // Default Android Hotspot SSID often "AndroidWifi" or "AndroidAP"
                    bssid: netsim_packets::ethernet::MacAddr::from([
                        0x02, 0x00, 0x00, 0x44, 0x55, 0x66,
                    ]),
                    channel: 6,
                    hw_mode: "g".to_string(),
                    wpa_passphrase: None,
                    beacon_interval: 100,
                    country_code: None,
                    dtim_period: 2,
                    hidden_ssid: false,
                    sae: false,
                    wmm_enabled: true,
                    enterprise_enabled: false,
                    mac_acl_mode: 0,
                    mac_acl_list: vec![],
                    ftm_responder_enabled: true,
                    position: netsim_model::device::Position::default(),
                };
                match ap_client.create_ap(config).await {
                    Ok(id) => log::info!("Created default AP with ID: {}", id),
                    Err(e) => log::error!("Failed to create default AP: {}", e),
                }
            }
        }
    }

    async fn on_shutdown(&mut self) {
        log::info!("WifiActor stopped");
    }

    async fn on_stream(
        &mut self,
        chip_id: ChipId,
        packet: bytes::Bytes,
        _ctx: &mut DynContext<ChipId>,
    ) {
        if chip_id == AP_ID {
            self.process_ap_packet(packet);
        } else if chip_id == SLIRP_ID {
            self.process_slirp_packet(packet);
        } else {
            self.process_guest_packet(chip_id.0, packet).await;
        }
    }

    async fn on_stream_closed(&mut self, id: ChipId, ctx: &mut DynContext<ChipId>) {
        log::info!("Stream closed for chip {id}");
        ctx.abort(id);
        if let Err(e) = self.handle_delete_impl(id, ctx).await {
            log::error!("Failed to delete chip {id} after stream closed: {e}");
        }
    }

    async fn on_task_closed(&mut self, id: ChipId, ctx: &mut DynContext<ChipId>) {
        log::info!("Sink task closed for chip {id}");
        ctx.remove_stream(id);
        if let Err(e) = self.handle_delete_impl(id, ctx).await {
            log::error!("Failed to delete chip {id} after sink task closed: {e}");
        }
    }
}

/// Creates an unbounded channel and wraps the receiver in a pinned BoxStream.
fn create_channel_stream<T>(
) -> (tokio::sync::mpsc::UnboundedSender<T>, futures::stream::BoxStream<'static, T>)
where
    T: Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    (tx, Box::pin(stream))
}
