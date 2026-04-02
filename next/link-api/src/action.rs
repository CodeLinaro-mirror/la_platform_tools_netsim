// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::{ChipId, ChipKind};

/// Defines the actions for the link actor.
#[derive(Debug, Clone)]
pub enum LinkAction {
    NotifyChipAdded(ChipId, ChipKind),
    NotifyChipRemoved(ChipId),
    Reset,
}
