// Copyright 2025 The Android Open Source Project

use link_api::{Link, LinkId};
use netsim_model::chip::{ChipId, ChipKind};
use std::collections::HashMap;

/// Link Entity.
///
/// Represents a connection between two chips.
/// Managed by `ResourceActor` in a HashMap.
#[derive(Debug, Clone, Default)]
pub struct LinkEntity {
    pub link: Link,
}

impl LinkEntity {
    pub fn new(link: Link) -> Self {
        Self { link }
    }
}

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
    pub lookup: HashMap<(ChipId, ChipId), LinkId>,
    /// Map of ChipId to ChipKind for validation
    pub chip_kind_map: HashMap<ChipId, ChipKind>,
    // TODO: Add radio_clients (e.g., bt_client) here
}

use crate::error::LinkError;
use actor_framework::ActorLifecycle;
use async_trait::async_trait;

#[async_trait]
impl ActorLifecycle for LinkActor {
    type Error = LinkError;
}
