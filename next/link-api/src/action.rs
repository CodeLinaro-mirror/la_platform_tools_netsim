// Copyright 2025 The Android Open Source Project

use netsim_model::chip::{ChipId, ChipKind};

/// Defines the actions for the link actor.
#[derive(Debug, Clone)]
pub enum LinkAction {
    NotifyChipAdded(ChipId, ChipKind),
    NotifyChipRemoved(ChipId),
}
