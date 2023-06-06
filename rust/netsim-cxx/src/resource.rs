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

use std::sync::RwLock;

use crate::captures::capture::Captures;
use crate::devices::devices_handler::Devices;
use crate::version::get_version;

/// Resource struct includes all the frontend resources for netsim.
/// This struct will be instantiated by netsimd and passed down to the
/// servers.
pub struct Resource {
    pub version: String,
    pub devices: RwLock<Devices>,
    pub captures: RwLock<Captures>,
}

impl Resource {
    pub fn new() -> Self {
        Self {
            version: get_version(),
            devices: RwLock::new(Devices::new()),
            captures: RwLock::new(Captures::new()),
        }
    }

    pub fn get_version_resource(self) -> String {
        self.version
    }

    pub fn get_devices_resource(self) -> RwLock<Devices> {
        self.devices
    }

    pub fn get_captures_resource(self) -> RwLock<Captures> {
        self.captures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let resource = Resource::new();
        assert_eq!(get_version(), resource.get_version_resource());
    }

    #[test]
    fn test_devices() {
        let resource = Resource::new();
        let devices = resource.get_devices_resource();
        assert!(!devices.is_poisoned());
    }

    #[test]
    fn test_captures() {
        let resource = Resource::new();
        let captures = resource.get_captures_resource();
        assert!(!captures.is_poisoned());
    }
}
