use std::{collections::HashMap, sync::Arc};

use actor_framework::DynContext;
use ap_actor::{shared::SharedKeyStore, ApClient};
use log::{debug, warn};
use netsim_model::{
    chip::{Chip, ChipId},
    stats::NetsimRadioStats,
};
use netsim_packets::ieee80211::{FrameDirection, Ieee80211};
use slirp_actor::SlirpClient;
use tokio::sync::mpsc::UnboundedSender;

use crate::{error::WifiError, medium::Medium};

/// ID for a Chip (Station)
pub type ChipIdType = u32;

#[derive(Debug)]
pub enum WifiReq {
    GetStatistics,
    Reset { id: ChipId },
}

/// Responses from the WifiActor (Output).
#[derive(Debug, Clone)]
pub enum WifiResponse {
    Ok,
    Statistics(Box<[NetsimRadioStats]>),
    Error(String),
}

#[derive(Debug)]
pub struct WifiActor {
    pub(crate) ap_client: Option<Arc<ApClient>>,
    pub(crate) slirp_client: Option<SlirpClient>,
    pub(crate) medium: Medium,
    pub(crate) active_chips: HashMap<ChipId, Chip>,
    pub(crate) senders: HashMap<ChipId, UnboundedSender<bytes::Bytes>>,
    pub(crate) shared_keys: Arc<SharedKeyStore>,
    // Output buffer for Medium to avoid allocations
    pub(crate) out_queue: Vec<(u32, bytes::Bytes)>,
    pub(crate) device_client: ::client::DeviceClient,
    // Channel to send frames TO the AP Actor (registered via ApClient)
    pub(crate) to_ap: Option<tokio::sync::mpsc::UnboundedSender<bytes::Bytes>>,
    // Channel to send frames TO the Slirp Actor (registered via SlirpClient)
    pub(crate) to_slirp: Option<tokio::sync::mpsc::UnboundedSender<bytes::Bytes>>,
}

impl WifiActor {
    pub fn new(
        ap_client: Option<Arc<ApClient>>,
        slirp_client: Option<SlirpClient>,
        device_client: ::client::DeviceClient,
    ) -> Self {
        let shared_keys = Arc::new(SharedKeyStore::new());
        let medium = Medium::new(
            shared_keys.clone(),
            crate::stats::WifiStats::default(),
            Arc::new(crate::DebugArgs::default()),
        );
        Self {
            ap_client,
            slirp_client,
            medium,
            active_chips: HashMap::new(),
            senders: HashMap::new(),
            shared_keys,
            out_queue: Vec::new(),
            device_client,
            to_ap: None,
            to_slirp: None,
        }
    }

    pub(crate) async fn handle_delete_impl(
        &mut self,
        id: ChipId,
        _ctx: &mut DynContext<WifiActor>,
    ) -> Result<(), WifiError> {
        if let Some(chip) = self.active_chips.remove(&id) {
            self.senders.remove(&id);
            self.medium.remove(id.0);

            // Notify DeviceService
            let dc = self.device_client.clone();
            let device_id = chip.device_id;
            // Notify DeviceService asynchronously
            tokio::spawn(async move {
                let _ = dc.notify_chip_removed(device_id, id).await;
            });

            Ok(())
        } else {
            Err(WifiError::Internal(format!("Chip {} not found", id)))
        }
    }

    pub(crate) fn flush_out_queue(&mut self) {
        for (chip_id, packet) in self.out_queue.drain(..) {
            if let Some(sender) = self.senders.get(&ChipId(chip_id)) {
                let _ = sender.send(packet);
            }
        }
    }

    // this is the input router
    pub(crate) async fn process_guest_packet(&mut self, chip_id: u32, packet: bytes::Bytes) {
        debug!("WifiActor: Packet from Guest (Chip {}) len {}", chip_id, packet.len());

        match self.medium.resolve_tx_packet(chip_id, &packet) {
            Ok(tx_state) => {
                // 1. Ack
                if let Err(e) = self.medium.ack_frame(chip_id, &tx_state.frame, &mut self.out_queue)
                {
                    warn!("Failed to ack frame: {:?}", e);
                }

                // 2. Infra (AP/Slirp) routing
                match tx_state.infra_target {
                    crate::medium::tx_packet_state::InfraTarget::Ap => {
                        debug!("ROUTING: Guest -> AP");
                        if let Some(to_ap) = &self.to_ap {
                            let _ = to_ap.send(bytes::Bytes::from(tx_state.get_ieee80211_bytes()));
                        }
                    }
                    crate::medium::tx_packet_state::InfraTarget::Slirp => {
                        debug!("ROUTING: Guest -> Slirp");
                        if let Some(to_slirp) = &self.to_slirp {
                            match tx_state.get_ieee80211().to_ieee8023() {
                                Ok(eth_frame) => {
                                    let _ = to_slirp.send(bytes::Bytes::from(eth_frame));
                                }
                                Err(e) => {
                                    warn!("WifiActor: Failed to convert to 802.3 for Slirp: {}", e);
                                }
                            }
                        }
                    }
                    crate::medium::tx_packet_state::InfraTarget::None => {
                        // Check for FTM Request (Peer-to-Peer Ranging)
                        if tx_state.stations {
                            // Check matching FTM Request
                            // TODO: Avoid parsing if possible, but we need to check Frame payload.
                            let frame_bytes = tx_state.get_ieee80211_bytes();
                            // Decode to check content
                            if let Ok(frame) = Ieee80211::decode(&frame_bytes) {
                                let da = frame.get_destination();
                                if let Some(peer_id) = self.medium.get_station_chip_id(&da) {
                                    // We found the target chip. Now get positions.
                                    // Initiator: chip_id
                                    // Responder: peer_id
                                    if let (Some(initiator), Some(responder)) = (
                                        self.active_chips.get(&ChipId(chip_id)),
                                        self.active_chips.get(&ChipId(peer_id)),
                                    ) {
                                        if let Some(responses) = crate::ftm::handle_ftm_request(
                                            &frame,
                                            &initiator.position,
                                            &responder.position,
                                        ) {
                                            debug!(
                                                "Simulated FTM Response from {} to {}",
                                                peer_id, chip_id
                                            );
                                            for resp in responses {
                                                self.out_queue.push((chip_id, resp));
                                            }
                                            // Suppress generic transmission.
                                            // Act as a Hardware Offload/Medium Interception to
                                            // ensure ONLY the simulated FTM response is sent.
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. Stations (Loopback/Peers)
                if tx_state.stations {
                    // optimized: transmit now takes references
                    if let Err(e) = self.medium.transmit(
                        &tx_state.frame,
                        tx_state.get_ieee80211(),
                        &mut self.out_queue,
                    ) {
                        warn!("Error queuing frame: {:?}", e);
                    }
                }
            }
            Err(e) => warn!("Error processing packet: {:?}", e),
        }
        self.flush_out_queue();
    }

    pub(crate) fn process_ap_packet(&mut self, packet: bytes::Bytes) {
        debug!("AP_PKT: len {}", packet.len());
        if !packet.is_empty() {
            let _ = self.medium.transmit_from_infra(&packet, &mut self.out_queue);
        }
        self.flush_out_queue();
    }

    pub(crate) fn process_slirp_packet(&mut self, packet: bytes::Bytes) {
        debug!("SLIRP_PKT: len {}", packet.len());

        if let Some(bssid) = self.shared_keys.get_bssid() {
            if let Ok(ieee80211) = Ieee80211::from_ieee8023(&packet, bssid, FrameDirection::FromAp)
            {
                if let Ok(bytes) = ieee80211.encode_to_vec() {
                    let _ = self
                        .medium
                        .transmit_from_infra(&bytes::Bytes::from(bytes), &mut self.out_queue);
                }
            } else {
                warn!("Failed to convert Slirp packet to 802.11");
            }
        } else {
            warn!("No BSSID available for Slirp packet conversion");
        }
        self.flush_out_queue();
    }
}
