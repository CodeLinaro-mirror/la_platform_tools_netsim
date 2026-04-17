use std::collections::HashMap;

use device_actor::DeviceClient;
use modem_rs::{modem_network::ModemNetworkInterface, ModemNetworkSimulator};
use netsim_model::chip::ChipId;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct ChipState {
    pub device_id: device_api::DeviceId,
}

pub struct CellActor {
    pub device_client: DeviceClient,
    pub controller: Box<dyn ModemNetworkInterface>,
    pub active_chips: HashMap<ChipId, ChipState>,
    pub event_receiver: Option<UnboundedReceiver<modem_rs::HostEvent>>,
}

impl CellActor {
    pub fn new(device_client: DeviceClient) -> Self {
        // Create channel for internal events loopback
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<modem_rs::HostEvent>();
        let controller = Box::new(ModemNetworkSimulator::new(event_tx));

        Self {
            device_client,
            controller,
            active_chips: HashMap::new(),
            event_receiver: Some(event_rx),
        }
    }
}
