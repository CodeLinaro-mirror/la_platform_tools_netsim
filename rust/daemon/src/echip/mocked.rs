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

use crate::devices::device::DeviceIdentifier;
use crate::echip::{EmulatedChip, SharedEmulatedChip};

use netsim_proto::common::ChipKind as ProtoChipKind;
use netsim_proto::model::Chip as ProtoChip;

use std::sync::Arc;

/// Parameters for creating Mocked chips
pub struct CreateParams {}

/// Mock struct is remained empty.
pub struct Mock {}

impl EmulatedChip for Mock {
    fn handle_request(&self, packet: &[u8]) {}

    fn reset(&self) {}

    fn get(&self) -> ProtoChip {
        ProtoChip::new()
    }

    fn patch(&self, chip: &ProtoChip) {}

    fn get_kind(&self) -> ProtoChipKind {
        ProtoChipKind::UNSPECIFIED
    }

    fn get_facade_id(&self) -> crate::devices::chip::FacadeIdentifier {
        todo!();
    }
}

/// Create a new MockedChip
pub fn new(create_params: &CreateParams, device_id: DeviceIdentifier) -> SharedEmulatedChip {
    Arc::new(Box::new(Mock {}))
}
