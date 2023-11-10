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

pub mod ble_beacon;
pub mod bluetooth;
pub mod emulated_chip;
pub mod mocked;
pub mod wifi;

pub use crate::echip::emulated_chip::new;
pub use crate::echip::emulated_chip::CreateParam;
pub use crate::echip::emulated_chip::EmulatedChip;
use std::sync::Arc;

pub type SharedEmulatedChip = Arc<Box<dyn EmulatedChip + Send + Sync>>;
