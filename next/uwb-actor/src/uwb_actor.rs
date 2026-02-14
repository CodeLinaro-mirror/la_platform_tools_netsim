// Copyright 2026 The Android Open Source Project

use std::{collections::HashMap, time::Duration};

use client::DeviceClient;
use netsim_model::chip::{Chip, ChipId};
use pica::{Handle, Pica, PicaCommand, PicaEvent, RangingEstimator, RangingMeasurement};
use tokio::sync::{broadcast, mpsc};

/// State associated with a single UWB chip.
#[derive(Clone)]
pub struct UwbChipState {
    /// The chip model.
    pub(super) chip: Chip,
    /// Mapping from chip ID to Pica handle.
    pub(super) pica_handle: Handle,
}

/// The UWB Actor responsible for managing UWB chips and their state.
pub struct UwbActor {
    /// Map of active chips.
    pub(super) chip_states: HashMap<ChipId, UwbChipState>,
    /// Inverse mapping for translating Pica events.
    pub(super) handle_to_chip: HashMap<Handle, ChipId>,
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
        let pica = Pica::new(Box::new(MockRangingEstimator), None);

        UwbActor {
            chip_states: HashMap::new(),
            handle_to_chip: HashMap::new(),
            device_client,
            pica_commands: pica.commands(),
            pica_on_tick_events: pica.events(),
            pica_connect_events: pica.events(),
            pica: Some(pica),
        }
    }
}

// TODO(b/458545089): implement real estimator
struct MockRangingEstimator;

impl RangingEstimator for MockRangingEstimator {
    fn estimate(&self, _left: &Handle, _right: &Handle) -> Option<RangingMeasurement> {
        Some(Default::default())
    }
}
