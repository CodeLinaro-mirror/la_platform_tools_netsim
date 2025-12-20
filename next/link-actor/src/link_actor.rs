// Copyright 2025 The Android Open Source Project

use link_api::Link;
use netsim_model::chip::{ChipId, ChipKind};
use std::collections::HashMap;

/// Context for LinkActor.
///
/// Holds the global state shared across all Link entities.
///
/// # State Management
/// - `lookup`: Maps (Sender, Receiver) to `LinkId` for fast upsert/retrieval.
/// - `chip_kind_map`: Maps ChipId to ChipKind for validation.
#[derive(Default)]
pub struct LinkActor {
    /// Index for fast lookup/upsert: (Sender, Receiver) -> LinkId
    pub(crate) chip_pairs: HashMap<(ChipId, ChipId), link_api::LinkId>,
    /// Map of ChipId to ChipKind for validation
    pub(crate) chip_kind_map: HashMap<ChipId, ChipKind>,
    /// Map of ChipKind to ChipClient for forwarding updates
    #[allow(dead_code)]
    pub(crate) chip_clients: HashMap<ChipKind, Box<dyn netsim_model::chip::ChipClient>>,
    pub(crate) links: HashMap<link_api::LinkId, Link>,
    pub(crate) next_id: u32,
}

impl LinkActor {
    pub fn new() -> Self {
        Self {
            chip_pairs: HashMap::new(),
            chip_kind_map: HashMap::new(),
            chip_clients: HashMap::new(),
            links: HashMap::new(),
            next_id: 1,
        }
    }
}
