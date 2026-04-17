use std::sync::atomic::{AtomicU16, Ordering};

use actor_framework::DynContext;
use ap_actor::shared::SharedKeyStore;
use log::{debug, warn};
use netsim_model::ChipId;
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
    seq: AtomicU16,
}

impl SlirpGateway {
    pub fn new(client: Option<SlirpClient>) -> Self {
        let (uplink_tx, uplink_rx) = tokio::sync::mpsc::unbounded_channel();
        let (downlink_tx, downlink_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sender: uplink_tx,
            pending_channels: Some((uplink_rx, downlink_tx, downlink_rx)),
            client,
            seq: AtomicU16::new(100),
        }
    }
}

#[async_trait::async_trait]
impl GatewayTrait for SlirpGateway {
    async fn send_80211(
        &self,
        _chip_id: ChipId,
        ieee80211: &Ieee80211,
    ) -> Result<usize, crate::error::WifiError> {
        // Drop QosNodata frames (keep-alives/null data) as they contain no payload
        // and cannot be converted to Ethernet.
        if ieee80211.is_qos_nodata() {
            return Ok(0);
        }

        ieee80211
            .to_ieee8023()
            .map_err(|e| {
                let fc = ieee80211.get_fc();
                let ftype = ieee80211.is_data();
                let stype = ieee80211.stype();
                crate::error::WifiError::Frame(format!(
                    "Slirp conversion failed: {}. Frame (Data: {}), Subtype: {}, FC: {:#06x}",
                    e, ftype, stype, fc
                ))
            })
            .and_then(|eth| {
                let payload_len = eth.len().saturating_sub(crate::gateway::ETHERNET_HEADER_LEN);
                self.sender.send(bytes::Bytes::from(eth)).map(|_| payload_len).map_err(|e| {
                    crate::error::WifiError::Network(format!("Slirp send failed: {}", e))
                })
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
        medium.wifi_stats.incr_network_packets_rx();

        let Some(bssid) = shared_keys.get_bssid() else {
            medium.wifi_stats.log_and_incr_err_count(&crate::error::WifiError::Network(
                "No BSSID available for Slirp packet conversion".to_string(),
            ));
            return;
        };

        medium.wifi_stats.record_download_bytes(
            packet.len().saturating_sub(crate::gateway::ETHERNET_HEADER_LEN),
        );

        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let res = Ieee80211::from_ieee8023_qos(&packet, bssid, FrameDirection::FromAp, true, seq)
            .map_err(|_| {
                crate::error::WifiError::Frame("Failed to convert Slirp packet to 802.11".into())
            });

        let ieee80211 = match res {
            Ok(i) => i,
            Err(e) => {
                medium.wifi_stats.log_and_incr_err_count(&e);
                return;
            }
        };

        let res = ieee80211.encode_to_vec().map_err(|_| {
            crate::error::WifiError::Frame("Failed to encode Slirp packet to 802.11".into())
        });
        match res {
            Ok(bytes) => {
                let res = medium.transmit_from_infra(&bytes::Bytes::from(bytes), out_queue);
                medium.wifi_stats.log_outcome(res, |_, _| {});
            }
            Err(e) => medium.wifi_stats.log_and_incr_err_count(&e),
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

    async fn on_chip_remove(&mut self, _chip_id: ChipId, _ctx: &mut DynContext<WifiActor>) {}

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
