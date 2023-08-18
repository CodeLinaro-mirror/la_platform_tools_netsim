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

use frontend_proto::common::ChipKind;

/// Helper function for translating u32 representation of ChipKind
pub(crate) fn int_to_chip_kind(kind: u32) -> ChipKind {
    match kind {
        1 => ChipKind::BLUETOOTH,
        2 => ChipKind::WIFI,
        3 => ChipKind::UWB,
        _ => ChipKind::UNSPECIFIED,
    }
}
