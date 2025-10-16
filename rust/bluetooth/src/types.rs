// Copyright 2023-2025 The Android Open Source Project

//! This module defines the data structures and traits used by the Bluetooth
//! manager.

use netsim_api::{chip_error::ChipError, chips::BluetoothMode};
use netsim_proto::model::Chip as ProtoChip;
use tokio::sync::oneshot;

/// A notification sent from a chip to the manager when it terminates unexpectedly.
#[derive(Debug)]
pub(crate) struct ChipDied {
    /// The ID of the chip that died.
    pub chip_id: u32,
}

pub(crate) struct ChipEntry {
    #[allow(dead_code)]
    pub(crate) bluetooth_mode: BluetoothMode,
    pub(crate) chip: Box<dyn EmulatedChip>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
}

/// A trait for emulated Bluetooth chips.
pub(crate) trait EmulatedChip: Send + Sync {
    /// Patches the state of the chip.
    fn update_chip(&mut self, update: ProtoChip) -> Result<ProtoChip, ChipError>;
    /// Returns the current state of the chip.
    fn get_chip(&self) -> Result<ProtoChip, ChipError>;
}
