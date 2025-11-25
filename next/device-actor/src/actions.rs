// Copyright (C) 2025 The Android Open Source Project

//! Device Actions
//!
//! This module defines the custom actions that can be performed on a Device actor,
//! and the results returned by those actions.
//!
//! Actions allow for domain-specific operations that go beyond standard CRUD
//! (Create, Read, Update, Delete) operations provided by the base actor framework.

use netsim_model::chip::ChipId;
use netsim_model::device::api::DeviceChipCreate;
use netsim_model::device::DeviceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceAction {
    Reset,
    NotifyChipRemoved(DeviceId, ChipId),
    AddChip(DeviceChipCreate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceActionResult {
    Success,
    ChipId(ChipId),
}
