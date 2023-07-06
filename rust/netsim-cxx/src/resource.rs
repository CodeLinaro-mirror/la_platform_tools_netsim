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

use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};

use crate::captures::capture::Captures;
use crate::devices::devices_handler::Devices;
use crate::version::get_version;

lazy_static! {
    static ref RESOURCES: RwLock<Resource> = RwLock::new(Resource::new());
}

/// Resource struct includes all the frontend resources for netsim.
/// This struct will be instantiated by netsimd and passed down to the
/// servers.
pub struct Resource {
    version: String,
    devices: Arc<RwLock<Devices>>,
    captures: Arc<RwLock<Captures>>,
}

impl Resource {
    pub fn new() -> Self {
        Self {
            version: get_version(),
            devices: Arc::new(RwLock::new(Devices::new())),
            captures: Arc::new(RwLock::new(Captures::new())),
        }
    }

    pub fn get_version_resource(self) -> String {
        self.version
    }
}

pub fn get_devices_resource() -> Arc<RwLock<Devices>> {
    RESOURCES.write().unwrap().devices.to_owned()
}

pub fn get_captures_resource() -> Arc<RwLock<Captures>> {
    RESOURCES.write().unwrap().captures.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let resource = Resource::new();
        assert_eq!(get_version(), resource.get_version_resource());
    }
}
