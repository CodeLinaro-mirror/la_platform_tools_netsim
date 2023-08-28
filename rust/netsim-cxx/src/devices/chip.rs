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
use crate::bluetooth as bluetooth_facade;
use crate::devices::id_factory::IdFactory;
use crate::wifi as wifi_facade;
use frontend_proto::common::ChipKind as ProtoChipKind;
use frontend_proto::model::Chip as ProtoChip;
use lazy_static::lazy_static;
use protobuf::EnumOrUnknown;
use std::sync::RwLock;

pub type ChipIdentifier = u32;
pub type FacadeIdentifier = u32;

const INITIAL_CHIP_ID: ChipIdentifier = 1000;

// Allocator for chip identifiers.
lazy_static! {
    static ref IDS: RwLock<IdFactory<ChipIdentifier>> =
        RwLock::new(IdFactory::new(INITIAL_CHIP_ID, 1));
}

pub struct Chip {
    pub id: ChipIdentifier,
    pub facade_id: Option<FacadeIdentifier>,
    pub kind: ProtoChipKind,
    pub name: String,
    // TODO: may not be necessary
    pub device_name: String,
    // These are patchable
    manufacturer: String,
    product_name: String,
}

impl Chip {
    fn new(
        id: ChipIdentifier,
        facade_id: Option<FacadeIdentifier>,
        kind: ProtoChipKind,
        name: &str,
        device_name: &str,
        manufacturer: &str,
        product_name: &str,
    ) -> Self {
        Self {
            id,
            facade_id,
            kind,
            name: name.to_string(),
            device_name: device_name.to_string(),
            manufacturer: manufacturer.to_string(),
            product_name: product_name.to_string(),
        }
    }

    /// Create the model protobuf
    pub fn get(&self) -> Result<ProtoChip, String> {
        let mut chip = ProtoChip::new();
        chip.kind = EnumOrUnknown::new(self.kind);
        chip.id = self.id;
        chip.name = self.name.clone();
        chip.manufacturer = self.manufacturer.clone();
        chip.product_name = self.product_name.clone();
        match (chip.kind.enum_value(), self.facade_id) {
            (Ok(ProtoChipKind::BLUETOOTH), Some(facade_id)) => {
                chip.set_bt(bluetooth_facade::bluetooth_get(facade_id));
            }
            (Ok(ProtoChipKind::BLUETOOTH_BEACON), Some(_)) => {
                chip.set_ble_beacon(bluetooth_facade::bluetooth_beacon_get(self.id)?);
            }
            (Ok(ProtoChipKind::WIFI), Some(facade_id)) => {
                chip.set_wifi(wifi_facade::wifi_get(facade_id));
            }
            (_, None) => {
                return Err(format!(
                    "Facade Id hasn't been added yet to frontend resource for chip_id: {}",
                    self.id
                ));
            }
            _ => {
                return Err(format!("Unknown chip kind: {:?}", chip.kind));
            }
        }
        Ok(chip)
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
        match self.facade_id {
            Some(facade_id) => {
                // Check both ChipKind and RadioKind fields, they should be consistent
                if self.kind == ProtoChipKind::BLUETOOTH && patch.has_bt() {
                    bluetooth_facade::bluetooth_patch(facade_id, patch.bt());
                    Ok(())
                } else if self.kind == ProtoChipKind::BLUETOOTH_BEACON
                    && patch.has_ble_beacon()
                    && patch.ble_beacon().bt.is_some()
                {
                    bluetooth_facade::bluetooth_beacon_patch(self.id, patch.ble_beacon())?;
                    Ok(())
                } else if self.kind == ProtoChipKind::WIFI && patch.has_wifi() {
                    wifi_facade::wifi_patch(facade_id, patch.wifi());
                    Ok(())
                } else {
                    Err(format!("Unknown chip kind or missing radio: {:?}", self.kind))
                }
            }
            None => Err(format!(
                "Facade Id hasn't been added yet to frontend resource for chip_id: {}",
                self.id
            )),
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        match (self.kind, self.facade_id) {
            (ProtoChipKind::BLUETOOTH, Some(facade_id)) => {
                bluetooth_facade::bluetooth_reset(facade_id);
                Ok(())
            }
            (ProtoChipKind::WIFI, Some(facade_id)) => {
                wifi_facade::wifi_reset(facade_id);
                Ok(())
            }
            (_, None) => Err(format!(
                "Facade Id hasn't been added yet to frontend resource for chip_id: {}",
                self.id
            )),
            _ => Err(format!("Unknown chip kind: {:?}", self.kind)),
        }
    }
}

/// Allocates a new chip with a facade_id.
pub fn chip_new(
    chip_kind: ProtoChipKind,
    chip_name: &str,
    device_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
) -> Result<Chip, String> {
    Ok(Chip::new(
        IDS.write().unwrap().next_id(),
        None,
        chip_kind,
        chip_name,
        device_name,
        chip_manufacturer,
        chip_product_name,
    ))
}

/// For testing
#[cfg(test)]
pub fn refresh_resource() {
    let mut ids = IDS.write().unwrap();
    ids.reset_id();
}
