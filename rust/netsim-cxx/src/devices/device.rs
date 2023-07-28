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

// Device.rs

use frontend_proto::model::State;
use protobuf::Message;

use crate::devices::chip;
use crate::devices::chip::Chip;
use crate::devices::chip::ChipIdentifier;
use crate::devices::chip::FacadeIdentifier;
use frontend_proto::common::ChipKind as ProtoChipKind;
use frontend_proto::model::Device as ProtoDevice;
use frontend_proto::model::Orientation as ProtoOrientation;
use frontend_proto::model::Position as ProtoPosition;
use std::collections::BTreeMap;

pub type DeviceIdentifier = u32;

pub struct Device {
    pub id: DeviceIdentifier,
    pub guid: String,
    pub name: String,
    visible: State,
    pub position: ProtoPosition,
    orientation: ProtoOrientation,
    pub chips: BTreeMap<ChipIdentifier, Chip>,
}
impl Device {
    pub fn new(id: DeviceIdentifier, guid: String, name: String) -> Self {
        Device {
            id,
            guid,
            name,
            visible: State::ON,
            position: ProtoPosition::new(),
            orientation: ProtoOrientation::new(),
            chips: BTreeMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct AddChipResult {
    pub device_id: DeviceIdentifier,
    pub chip_id: ChipIdentifier,
    pub facade_id: FacadeIdentifier,
}

impl Device {
    pub fn get(&self) -> Result<ProtoDevice, String> {
        let mut device = ProtoDevice::new();
        device.id = self.id;
        device.name = self.name.clone();
        device.visible = self.visible.into();
        device.position = protobuf::MessageField::from(Some(self.position.clone()));
        device.orientation = protobuf::MessageField::from(Some(self.orientation.clone()));
        for chip in self.chips.values() {
            device.chips.push(chip.get()?);
        }
        Ok(device)
    }

    /// Patch a device and its chips.
    pub fn patch(&mut self, patch: &ProtoDevice) -> Result<(), String> {
        let patch_visible = patch.visible.enum_value_or_default();
        if patch_visible != State::UNKNOWN {
            self.visible = patch_visible;
        }
        if patch.position.is_some() {
            self.position.clone_from(&patch.position);
        }
        if patch.orientation.is_some() {
            self.orientation.clone_from(&patch.orientation);
        }
        // iterate over patched ProtoChip entries and patch matching chip
        for patch_chip in patch.chips.iter() {
            let mut patch_chip_kind = patch_chip.kind.enum_value_or_default();
            // Check if chip is given when kind is not given.
            // TODO: Fix patch device request body in CLI to include ChipKind, and remove if block below.
            if patch_chip_kind == ProtoChipKind::UNSPECIFIED {
                if patch_chip.has_bt() {
                    patch_chip_kind = ProtoChipKind::BLUETOOTH;
                } else if patch_chip.has_wifi() {
                    patch_chip_kind = ProtoChipKind::WIFI;
                } else if patch_chip.has_uwb() {
                    patch_chip_kind = ProtoChipKind::UWB;
                } else {
                    break;
                }
            }
            let patch_chip_name = &patch_chip.name;
            // Find the matching chip and patch the proto chip
            for chip in self.chips.values_mut() {
                if (patch_chip_name.is_empty() || chip.name.eq(patch_chip_name))
                    && chip.kind == patch_chip_kind
                {
                    chip.patch(patch_chip)?;
                    break; // next proto chip
                }
            }
        }
        Ok(())
    }

    /// Remove a chip from a device.
    pub fn remove_chip(
        &mut self,
        chip_id: ChipIdentifier,
    ) -> Result<(Option<FacadeIdentifier>, ProtoChipKind), String> {
        let (facade_id, kind) = {
            if let Some(chip) = self.chips.get_mut(&chip_id) {
                (chip.facade_id, chip.kind)
            } else {
                return Err(format!("RemoveChip chip id {chip_id} not found"));
            }
        };
        match self.chips.remove(&chip_id) {
            Some(_) => Ok((facade_id, kind)),
            None => Err(format!("Key {chip_id} not found in Hashmap")),
        }
    }

    pub fn add_chip(
        &mut self,
        device_name: &str,
        chip_kind: ProtoChipKind,
        chip_name: &str,
        chip_manufacturer: &str,
        chip_product_name: &str,
    ) -> Result<(DeviceIdentifier, ChipIdentifier), String> {
        for chip in self.chips.values() {
            if chip.kind == chip_kind && chip.name == chip_name {
                return Err(format!("Device::AddChip - duplicate at id {}, skipping.", chip.id));
            }
        }
        let chip = chip::chip_new(
            chip_kind,
            chip_name,
            device_name,
            chip_manufacturer,
            chip_product_name,
        )?;
        let chip_id = chip.id;
        self.chips.insert(chip_id, chip);
        Ok((self.id, chip_id))
    }

    /// Reset a device to its default state.
    pub fn reset(&mut self) -> Result<(), String> {
        self.visible = State::ON;
        self.position.clear();
        self.orientation.clear();
        for chip in self.chips.values_mut() {
            chip.reset()?;
        }
        Ok(())
    }
}
