// Copyright (C) 2025 The Android Open Source Project

//! Device Actions
//!
//! This module defines the custom actions that can be performed on a Device actor,
//! and the results returned by those actions.
//!
//! Actions allow for domain-specific operations that go beyond standard CRUD
//! (Create, Read, Update, Delete) operations provided by the base actor framework.

use netsim_model::chip::{ChipId, PacketSink, PacketStream};
use netsim_model::device::api::DeviceChipCreate;
use netsim_model::device::DeviceId;
use serde::{Deserialize, Serialize};

use std::fmt;

pub enum DeviceAction {
    Reset,
    NotifyChipRemoved(DeviceId, ChipId),
    AddChip {
        chip_config: DeviceChipCreate,
        packet_stream: Option<PacketStream>,
        packet_sink: Option<PacketSink>,
    },
}

impl fmt::Debug for DeviceAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceAction::Reset => write!(f, "Reset"),
            DeviceAction::NotifyChipRemoved(device_id, chip_id) => {
                f.debug_tuple("NotifyChipRemoved").field(device_id).field(chip_id).finish()
            }
            DeviceAction::AddChip { chip_config, .. } => f
                .debug_struct("AddChip")
                .field("chip_config", chip_config)
                .field("packet_stream", &"Option<PacketStream>")
                .field("packet_sink", &"Option<PacketSink>")
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceActionResult {
    Success,
    ChipId(ChipId),
}
