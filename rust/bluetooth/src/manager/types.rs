// Copyright 2023-2025 The Android Open Source Project

//! This module defines the data structures and traits used by the Bluetooth
//! manager.

use crate::error::ChipError;
use netsim_api::{ChipPatch, CreateChipParams, DeleteChipParams, GetChipParams, PatchChipParams};
use tokio::sync::oneshot;

/// A reply channel for sending the result of an operation back to the caller.
pub(crate) type Responder<T> = oneshot::Sender<Result<T, ChipError>>;

/// A command sent to the `BluetoothManager`.
#[derive(Debug)]
pub enum BluetoothCommand {
    /// Create a new chip.
    CreateChip { params: CreateChipParams, responder: Responder<u32> },
    /// Patch an existing chip.
    PatchChip { params: PatchChipParams, responder: Responder<ChipState> },
    /// Get the state of a chip.
    GetChip { params: GetChipParams, responder: Responder<ChipState> },
    /// Delete a chip.
    DeleteChip { params: DeleteChipParams, responder: Responder<()> },
}

/// A notification sent from a chip to the manager when it terminates unexpectedly.
#[derive(Debug)]
pub(crate) struct ChipDied {
    /// The ID of the chip that died.
    pub chip_id: u32,
}

pub(crate) struct ChipEntry {
    pub(crate) chip: Box<dyn EmulatedChip>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
}

/// A trait for emulated Bluetooth chips.
pub(crate) trait EmulatedChip: Send + Sync {
    /// Patches the state of the chip.
    fn patch_chip(&mut self, patch: ChipPatch) -> Result<ChipState, ChipError>;
    /// Returns the current state of the chip.
    fn get_chip(&self) -> Result<ChipState, ChipError>;
}

/// The state of a chip.
#[derive(Debug, Clone)]
pub struct ChipState {
    /// The ID of the chip.
    pub id: u32,
    // TODO: Add other chip state fields, e.g., name, position, etc.
}
