use actor_framework::DynContext;
use ap_actor::shared::SharedKeyStore;
use log::{debug, warn};
use netsim_model::chip::ChipId;
use netsim_packets::ieee80211::{FrameDirection, Ieee80211};
use slirp_actor::SlirpClient;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    gateway::GatewayTrait,
    medium::Medium,
    wifi_actor::{SlirpPendingRequest, WifiActor},
};

/// Gateway implementation for User-Mode Networking (Slirp).
///
/// This is the *default* gateway when no TAP interface is configured.
/// It routes traffic to the `SlirpActor`, which wraps `libslirp` to provide
/// network connectivity without requiring special privileges or kernel
/// interfaces.
#[derive(Debug)]
pub struct SlirpGateway {
    sender: UnboundedSender<bytes::Bytes>,
    pending_channels: Option<SlirpPendingRequest>,
    client: Option<SlirpClient>,
}

impl SlirpGateway {
    pub fn new(client: Option<SlirpClient>) -> Self {
        let (uplink_tx, uplink_rx) = tokio::sync::mpsc::unbounded_channel();
        let (downlink_tx, downlink_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sender: uplink_tx,
            pending_channels: Some((uplink_rx, downlink_tx, downlink_rx)),
            client,
        }
    }
}

#[async_trait::async_trait]
impl GatewayTrait for SlirpGateway {
    async fn send_80211(&self, _chip_id: ChipId, ieee80211: &Ieee80211) -> bool {
        // Drop QosNodata frames (keep-alives/null data) as they contain no payload
        // and cannot be converted to Ethernet.
        if ieee80211.is_qos_nodata() {
            return true;
        }

        ieee80211
            .to_ieee8023()
            .map(|eth| {
                let _ = self.sender.send(bytes::Bytes::from(eth));
                true
            })
            .unwrap_or_else(|e| {
                let fc = ieee80211.get_fc();
                let ftype = ieee80211.is_data();
                let stype = ieee80211.stype();
                warn!(
                    "WifiActor: Slirp conversion failed: {}. Frame (Data: {}), Subtype: {}, FC: {:#06x}",
                    e, ftype, stype, fc
                );
                false
            })
    }

    fn should_handle(&self, chip_id: ChipId) -> bool {
        chip_id == crate::lifecycle::SLIRP_ID
    }

    fn handle_incoming(
        &self,
        _chip_id: ChipId,
        packet: bytes::Bytes,
        medium: &mut Medium,
        shared_keys: &SharedKeyStore,
        out_queue: &mut Vec<(u32, bytes::Bytes)>,
    ) {
        debug!("SLIRP_PKT: len {}", packet.len());
        if let Some(bssid) = shared_keys.get_bssid() {
            if let Ok(ieee80211) = Ieee80211::from_ieee8023(&packet, bssid, FrameDirection::FromAp)
            {
                if let Ok(bytes) = ieee80211.encode_to_vec() {
                    let _ = medium.transmit_from_infra(&bytes::Bytes::from(bytes), out_queue);
                }
            } else {
                warn!("Failed to convert Slirp packet to 802.11");
            }
        } else {
            warn!("No BSSID available for Slirp packet conversion");
        }
    }

    async fn on_start(&mut self, ctx: &mut DynContext<WifiActor>) {
        if let Some(client) = self.client.as_ref() {
            if let Some((uplink_rx, downlink_tx, downlink_rx)) = self.pending_channels.take() {
                debug!("Registering Slirp channels in on_start");
                // Register with SlirpActor
                let stream =
                    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(uplink_rx));
                if let Err(e) = client.register(stream, downlink_tx).await {
                    warn!("Failed to register with SlirpActor: {}", e);
                }

                // Add downlink stream to context
                ctx.add_stream(
                    crate::lifecycle::SLIRP_ID,
                    Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(downlink_rx)),
                );
            }
        }
    }

    async fn on_chip_create(&mut self, _chip_id: ChipId, _ctx: &mut DynContext<WifiActor>) {
        // No-op for Slirp
    }

    async fn on_chip_remove(&mut self, _chip_id: ChipId, _ctx: &mut DynContext<WifiActor>) {
        // No-op for Slirp
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
