// Copyright 2026 The Android Open Source Project

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use client::DeviceClient;
use netsim_model::chip::{Chip, ChipId};
use pica::{Handle, Pica, PicaEvent, RangingEstimator, RangingMeasurement};
use tokio::{sync::broadcast, task::JoinHandle};

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
    /// The Pica simulator instance.
    /// TODO(b/483089918): use lock-free form of Pica API
    pub(super) pica: Arc<Mutex<Pica>>,
    /// The join handle for the Pica run loop.
    pub(super) pica_task: Option<JoinHandle<Result<(), anyhow::Error>>>,
    /// Used to detect when a chip disconnects
    pub(super) pica_events: broadcast::Receiver<PicaEvent>,
}

impl UwbActor {
    /// Used to periodically check for stream closures.
    pub const TICK_INTERVAL: Duration = Duration::from_millis(100);

    pub fn new(device_client: DeviceClient) -> Self {
        let pica = Pica::new(Box::new(MockRangingEstimator), None);
        let events = pica.events();

        UwbActor {
            chip_states: HashMap::new(),
            handle_to_chip: HashMap::new(),
            device_client,
            pica_events: events,
            pica: Arc::new(Mutex::new(pica)),
            pica_task: None,
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
