// Copyright 2026 The Android Open Source Project

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use client::DeviceClient;
use netsim_model::chip::{Chip, ChipId};
use pica::{Handle, Pica, RangingEstimator, RangingMeasurement};
use tokio::{sync::mpsc, task::JoinHandle};

/// State associated with a single UWB chip.
#[derive(Clone)]
pub struct UwbChipState {
    /// The chip model.
    pub(super) chip: Chip,
    /// Sender for forwarding UCI packets to Pica.
    pub(super) pica_sender: mpsc::Sender<bytes::Bytes>,
    /// Mapping from chip ID to Pica handle.
    pub(super) pica_handle: Handle,
}

/// The UWB Actor responsible for managing UWB chips and their state.
pub struct UwbActor {
    /// Map of active chips.
    pub(super) chip_states: HashMap<ChipId, UwbChipState>,
    /// Client for interacting with the device actor.
    pub(super) device_client: DeviceClient,
    /// The Pica simulator instance.
    /// TODO(b/483089918): use lock-free form of Pica API
    pub(super) pica: Arc<Mutex<Pica>>,
    /// The join handle for the Pica run loop.
    pub(super) pica_task: Option<JoinHandle<Result<(), anyhow::Error>>>,
}

impl UwbActor {
    pub fn new(device_client: DeviceClient) -> Self {
        let pica = Arc::new(Mutex::new(Pica::new(Box::new(MockRangingEstimator), None)));
        UwbActor { chip_states: HashMap::new(), device_client, pica, pica_task: None }
    }
}

// TODO(b/458545089): implement real estimator
struct MockRangingEstimator;

impl RangingEstimator for MockRangingEstimator {
    fn estimate(&self, _left: &Handle, _right: &Handle) -> Option<RangingMeasurement> {
        Some(Default::default())
    }
}
