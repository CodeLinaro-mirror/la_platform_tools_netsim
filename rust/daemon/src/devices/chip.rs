// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// A Chip is a generic emulated radio that connects to Chip Facade
/// library.
///
/// The chip facade is a library that implements the controller protocol.
///
use crate::devices::id_factory::IdFactory;
use crate::echip::SharedEmulatedChip;
use lazy_static::lazy_static;
use log::warn;
use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::configuration::Controller as ProtoController;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::NetsimRadioStats as ProtoRadioStats;
use protobuf::EnumOrUnknown;
use std::sync::RwLock;
use std::time::Instant;

pub type ChipIdentifier = u32;
pub type FacadeIdentifier = u32;

const INITIAL_CHIP_ID: ChipIdentifier = 1000;

// Allocator for chip identifiers.
lazy_static! {
    static ref IDS: RwLock<IdFactory<ChipIdentifier>> =
        RwLock::new(IdFactory::new(INITIAL_CHIP_ID, 1));
}

pub struct CreateParams {
    pub kind: ProtoChipKind,
    pub address: String,
    pub name: Option<String>,
    pub manufacturer: String,
    pub product_name: String,
    pub bt_properties: Option<ProtoController>, // TODO: move to echip CreateParams
}

/// Chip contains the common information for each Chip/Controller.
/// Radio-specific information is contained in the emulated_chip.
pub struct Chip {
    pub id: ChipIdentifier,
    pub emulated_chip: Option<SharedEmulatedChip>,
    pub kind: ProtoChipKind,
    pub address: String,
    pub name: String,
    // TODO: may not be necessary
    pub device_name: String,
    // These are patchable
    pub manufacturer: String,
    pub product_name: String,
    pub start: Instant,
}

impl Chip {
    // Use an Option here so that the Chip can be created and
    // inserted into the Device prior to creation of the echip.
    // Any Chip with an emulated_chip == None is temporary.
    // Creating the echip first required holding a Chip+Device lock through
    // initialization which caused a deadlock under certain (rare) conditions.
    fn new(id: ChipIdentifier, device_name: &str, create_params: &CreateParams) -> Self {
        Self {
            id,
            emulated_chip: None,
            kind: create_params.kind,
            address: create_params.address.clone(),
            name: create_params.name.clone().unwrap_or(format!("chip-{id}")),
            device_name: device_name.to_string(),
            manufacturer: create_params.manufacturer.clone(),
            product_name: create_params.product_name.clone(),
            start: Instant::now(),
        }
    }

    // Get the stats protobuf for a chip controller instance.
    //
    // This currently wraps the chip "get" facade method because the
    // counts are phy level. We need a vec since Bluetooth reports
    // stats for BLE and CLASSIC.
    pub fn get_stats(&self) -> Vec<ProtoRadioStats> {
        match &self.emulated_chip {
            Some(emulated_chip) => emulated_chip.get_stats(self.start.elapsed().as_secs()),
            None => {
                warn!("EmulatedChip hasn't been instantiated yet for chip_id {}", self.id);
                Vec::<ProtoRadioStats>::new()
            }
        }
    }

    /// Create the model protobuf
    pub fn get(&self) -> Result<ProtoChip, String> {
        let mut proto_chip = self
            .emulated_chip
            .as_ref()
            .map(|c| c.get())
            .ok_or(format!("EmulatedChip hasn't been instantiated yet for chip_id {}", self.id))?;
        proto_chip.kind = EnumOrUnknown::new(self.kind);
        proto_chip.id = self.id;
        proto_chip.name = self.name.clone();
        proto_chip.manufacturer = self.manufacturer.clone();
        proto_chip.product_name = self.product_name.clone();
        Ok(proto_chip)
    }

    /// Patch processing for the chip. Validate and move state from the patch
    /// into the chip changing the ChipFacade as needed.
    pub fn patch(&mut self, patch: &ProtoChip) -> Result<(), String> {
        if !patch.manufacturer.is_empty() {
            self.manufacturer = patch.manufacturer.clone();
        }
        if !patch.product_name.is_empty() {
            self.product_name = patch.product_name.clone();
        }
        self.emulated_chip
            .as_ref()
            .map(|c| c.patch(patch))
            .ok_or(format!("EmulatedChip hasn't been instantiated yet for chip_id {}", self.id))
    }

    pub fn reset(&mut self) -> Result<(), String> {
        self.emulated_chip
            .as_ref()
            .map(|c| c.reset())
            .ok_or(format!("EmulatedChip hasn't been instantiated yet for chip_id {}", self.id))
    }
}

/// Allocates a new chip with a facade_id.
pub fn chip_new(device_name: &str, create_params: &CreateParams) -> Result<Chip, String> {
    let id = IDS.write().unwrap().next_id();
    Ok(Chip::new(id, device_name, create_params))
}
