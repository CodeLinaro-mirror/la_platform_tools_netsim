// Copyright 2026 The Android Open Source Project

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use device_actor::DeviceClient;
use netsim_model::chip::{Chip, ChipId, ChipUpdate, ChipVariant, ChipVariantUpdate};
use pica::{Handle, Pica, PicaCommand, PicaEvent};
use tokio::sync::{broadcast, mpsc};
use tracing::warn;

/// State associated with a single UWB chip.
#[derive(Clone)]
pub(crate) struct UwbChipState {
    /// The chip model.
    pub(super) chip: Chip,
}

impl UwbChipState {
    pub(super) fn apply(&mut self, update: ChipUpdate) {
        if let Some(pos) = update.position {
            self.chip.position = pos;
        }
        if let Some(orient) = update.orientation {
            self.chip.orientation = orient;
        }
        match (update.variant, &mut self.chip.variant) {
            (Some(ChipVariantUpdate::Uwb(uwb_update)), Some(ChipVariant::Uwb(uwb_radio))) => {
                uwb_update.radio.apply(&mut uwb_radio.radio);
            }
            (Some(other), _) => {
                warn!("Received unexpected update for chip {}: {other:?}", self.chip.id);
            }
            (None, _) => {}
        }
    }
}

/// The UWB Actor responsible for managing UWB chips and their state.
pub struct UwbActor {
    /// Map of active chips shared with
    /// [UwbRangingEstimator](super::ranging_estimator::UwbRangingEstimator).
    pub(super) chip_states: Arc<RwLock<HashMap<Handle, UwbChipState>>>,
    /// Map from chip ID to Pica handle, used for actor lookups.
    pub(super) chip_to_handle: HashMap<ChipId, Handle>,
    /// Client for interacting with the device actor.
    pub(super) device_client: DeviceClient,
    /// Pica simulator, present only prior to actor lifecycle `on_start`.
    pub(super) pica: Option<Pica>,
    /// Pica command queue
    pub(super) pica_commands: mpsc::Sender<PicaCommand>,
    /// Used to detect when a chip disconnects due to a stream closure.
    pub(super) pica_on_tick_events: broadcast::Receiver<PicaEvent>,
    /// Used to detect when a chip is connected after [`PicaCommand::Connect`].
    pub(super) pica_connect_events: broadcast::Receiver<PicaEvent>,
}

impl UwbActor {
    /// Used to periodically check for stream closures.
    pub const TICK_INTERVAL: Duration = Duration::from_millis(100);

    pub fn new(device_client: DeviceClient) -> Self {
        let chip_states = Arc::new(RwLock::new(HashMap::new()));
        let pica = Pica::new(
            Box::new(crate::ranging_estimator::UwbRangingEstimator::new(chip_states.clone())),
            None,
        );

        UwbActor {
            chip_states,
            chip_to_handle: HashMap::new(),
            device_client,
            pica_commands: pica.commands(),
            pica_on_tick_events: pica.events(),
            pica_connect_events: pica.events(),
            pica: Some(pica),
        }
    }
}
