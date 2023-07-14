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
use log::info;
use protobuf::EnumOrUnknown;

use std::sync::RwLock;

use crate::devices::device::DeviceIdentifier;

pub type ChipIdentifier = i32;
pub type FacadeIdentifier = u32;

const INITIAL_CHIP_ID: ChipIdentifier = 1000;

// Allocator for chip identifiers.
lazy_static! {
    static ref IDS: RwLock<IdFactory<ChipIdentifier>> =
        RwLock::new(IdFactory::new(INITIAL_CHIP_ID, 1));
}

pub struct Chip {
    pub id: ChipIdentifier,
    pub facade_id: FacadeIdentifier,
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
        facade_id: FacadeIdentifier,
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
        match chip.kind.enum_value() {
            Ok(ProtoChipKind::BLUETOOTH) => {
                chip.set_bt(bluetooth_facade::bluetooth_get(self.facade_id));
            }
            Ok(ProtoChipKind::BLUETOOTH_BEACON) => {
                chip.set_bt(bluetooth_facade::bluetooth_get(self.facade_id));
            }
            Ok(ProtoChipKind::WIFI) => {
                chip.set_wifi(wifi_facade::wifi_get(self.facade_id));
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
        // Check both ChipKind and RadioKind fields, they should be consistent
        if self.kind == ProtoChipKind::BLUETOOTH && patch.has_bt() {
            bluetooth_facade::bluetooth_patch(self.facade_id, patch.bt());
            Ok(())
        } else if self.kind == ProtoChipKind::BLUETOOTH_BEACON
            && patch.has_ble_beacon()
            && patch.ble_beacon().bt.is_some()
        {
            bluetooth_facade::bluetooth_patch(
                self.facade_id,
                &patch.ble_beacon().bt.clone().unwrap(),
            );
            Ok(())
        } else if self.kind == ProtoChipKind::WIFI && patch.has_wifi() {
            wifi_facade::wifi_patch(self.facade_id, patch.wifi());
            Ok(())
        } else {
            Err(format!("Unknown chip kind or missing radio: {:?}", self.kind))
        }
    }

    pub fn remove(&mut self) -> Result<(), String> {
        match self.kind {
            ProtoChipKind::BLUETOOTH => {
                bluetooth_facade::bluetooth_remove(self.facade_id);
                info!("Removed bt chip with facade_id: {}", self.facade_id);
                Ok(())
            }
            ProtoChipKind::WIFI => {
                wifi_facade::wifi_remove(self.facade_id);
                info!("Removed wifi chip with facade_id: {}", self.facade_id);
                Ok(())
            }
            _ => Err(format!("Unknown chip kind: {:?}", self.kind)),
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        match self.kind {
            ProtoChipKind::BLUETOOTH => {
                bluetooth_facade::bluetooth_reset(self.facade_id);
                Ok(())
            }
            ProtoChipKind::WIFI => {
                wifi_facade::wifi_reset(self.facade_id);
                Ok(())
            }
            _ => Err(format!("Unknown chip kind: {:?}", self.kind)),
        }
    }
}

/// Allocates a new chip with a facade_id.
pub fn chip_new(
    device_id: DeviceIdentifier,
    chip_kind: ProtoChipKind,
    chip_name: &str,
    device_name: &str,
    chip_manufacturer: &str,
    chip_product_name: &str,
) -> Result<Chip, String> {
    let chip_id = IDS.write().unwrap().next_id();
    let chip_id_u32 = u32::try_from(chip_id).map_err(|err| err.to_string())?;
    let device_id_u32 = u32::try_from(device_id).map_err(|err| err.to_string())?;
    let facade_id = match chip_kind {
        ProtoChipKind::BLUETOOTH => bluetooth_facade::bluetooth_add(device_id_u32),
        ProtoChipKind::BLUETOOTH_BEACON => bluetooth_facade::beacon::bluetooth_beacon_add(
            device_id_u32,
            chip_id_u32,
            "beacon".to_string(),
            chip_name.to_string(),
        ),
        ProtoChipKind::WIFI => wifi_facade::wifi_add(device_id_u32),
        _ => return Err(format!("Unknown chip kind: {:?}", chip_kind)),
    };
    Ok(Chip::new(
        chip_id,
        facade_id,
        chip_kind,
        chip_name,
        device_name,
        chip_manufacturer,
        chip_product_name,
    ))
}

/// For testing
#[cfg(all(test))]
pub fn refresh_resource() {
    let mut ids = IDS.write().unwrap();
    ids.reset_id();
}
