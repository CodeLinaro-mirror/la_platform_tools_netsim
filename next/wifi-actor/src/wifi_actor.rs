use crate::error::WifiError;
use crate::medium::Medium;
use actor_framework::DynContext;
use ap_actor::{shared::SharedKeyStore, ApClientTrait};
use netsim_model::chip::{Chip, ChipId};
use slirp_actor::SlirpClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

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
    Statistics(Box<[netsim_model::stats::NetsimRadioStats]>),
    Error(String),
}

#[derive(Debug)]
pub struct WifiActor {
    pub(crate) ap_client: Option<Arc<dyn ApClientTrait>>, // Changed type here
    pub(crate) slirp_client: Option<SlirpClient>,
    pub(crate) medium: Medium,
    pub(crate) active_chips: HashMap<ChipId, Chip>,
    pub(crate) senders: HashMap<ChipId, UnboundedSender<bytes::Bytes>>,
    pub(crate) shared_keys: Arc<SharedKeyStore>,
    // Output buffer for Medium to avoid allocations
    pub(crate) out_queue: Vec<(u32, bytes::Bytes)>,
    pub(crate) device_client: ::client::DeviceClient,
    pub(crate) create_default_ap: bool,
    // Channel to send frames TO the AP Actor (registered via ApClient)
    pub(crate) to_ap: Option<tokio::sync::mpsc::UnboundedSender<bytes::Bytes>>,
    // Channel to send frames TO the Slirp Actor (registered via SlirpClient)
    pub(crate) to_slirp: Option<tokio::sync::mpsc::UnboundedSender<bytes::Bytes>>,
}

impl WifiActor {
    pub fn new(
        ap_client: Option<Arc<dyn ApClientTrait>>, // Changed type here
        slirp_client: Option<SlirpClient>,
        device_client: ::client::DeviceClient,
        create_default_ap: bool,
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
            create_default_ap,
            to_ap: None,
            to_slirp: None,
        }
    }

    pub(crate) async fn handle_delete_impl(
        &mut self,
        id: ChipId,
        _ctx: &mut DynContext<ChipId>,
    ) -> Result<(), WifiError> {
        if let Some(chip) = self.active_chips.remove(&id) {
            self.senders.remove(&id);
            self.medium.remove(id.0);

            // Notify DeviceService
            let dc = self.device_client.clone();
            let device_id = chip.device_id;
            // Spawn notification to avoid blocking Actor action loop?
            // Actually handle_delete is async, we can await if acceptable or spawn.
            // BluetoothActor spawns.
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
        match self.medium.resolve_tx_packet(chip_id, &packet) {
            Ok(tx_state) => {
                // 1. Ack
                if let Err(e) = self.medium.ack_frame(chip_id, &tx_state.frame, &mut self.out_queue)
                {
                    log::error!("Failed to ack frame: {:?}", e);
                }

                // 2. Infra (AP/Slirp) routing
                match tx_state.infra_target {
                    crate::medium::tx_packet_state::InfraTarget::Ap => {
                        if let Some(to_ap) = &self.to_ap {
                            let _ = to_ap.send(bytes::Bytes::from(tx_state.get_ieee80211_bytes()));
                        }
                    }
                    crate::medium::tx_packet_state::InfraTarget::Slirp => {
                        if let Some(to_slirp) = &self.to_slirp {
                            if let Ok(eth_frame) = tx_state.get_ieee80211().to_ieee8023() {
                                let _ = to_slirp.send(bytes::Bytes::from(eth_frame));
                            }
                        }
                    }
                    crate::medium::tx_packet_state::InfraTarget::None => {}
                }

                // 3. Stations (Loopback/Peers)
                if tx_state.stations {
                    // optimized: transmit now takes references
                    if let Err(e) = self.medium.transmit(
                        &tx_state.frame,
                        tx_state.get_ieee80211(),
                        &mut self.out_queue,
                    ) {
                        log::error!("Error queuing frame: {:?}", e);
                    }
                }
            }
            Err(e) => log::error!("Error processing packet: {:?}", e),
        }
        self.flush_out_queue();
    }

    pub(crate) fn process_ap_packet(&mut self, packet: bytes::Bytes) {
        if !packet.is_empty() {
            let _ = self.medium.transmit_from_infra(&packet, &mut self.out_queue);
        }
        self.flush_out_queue();
    }

    pub(crate) fn process_slirp_packet(&mut self, packet: bytes::Bytes) {
        if let Some(bssid) = self.shared_keys.get_bssid() {
            if let Ok(ieee80211) =
                netsim_packets::ieee80211::Ieee80211::from_ieee8023(&packet, bssid)
            {
                if let Ok(from_ap) = ieee80211.into_from_ap() {
                    // TryInto is needed, ensure it is available or use strict path
                    if let Ok(frame_converted) =
                        TryInto::<netsim_packets::ieee80211::Ieee80211>::try_into(from_ap)
                    {
                        if let Ok(bytes) = frame_converted.encode_to_vec() {
                            let _ = self.medium.transmit_from_infra(
                                &bytes::Bytes::from(bytes),
                                &mut self.out_queue,
                            );
                        }
                    }
                }
            }
        }
        self.flush_out_queue();
    }
}
