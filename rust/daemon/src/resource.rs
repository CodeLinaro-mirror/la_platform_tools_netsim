// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, OnceLock, RwLock};

use crate::captures::capture::Captures;
use crate::version::get_version;

static RESOURCES: OnceLock<Resource> = OnceLock::new();

/// Resource struct includes all the global and possibly shared
/// resources for netsim.  Each field within Resource should be an Arc
/// protected by a RwLock or Mutex.
pub struct Resource {
    version: String,
    captures: Arc<RwLock<Captures>>,
}

impl Resource {
    pub fn new() -> Self {
        Self { version: get_version(), captures: Arc::new(RwLock::new(Captures::new())) }
    }

    #[allow(dead_code)]
    pub fn get_version_resource(self) -> String {
        self.version
    }
}

pub fn clone_captures() -> Arc<RwLock<Captures>> {
    Arc::clone(&RESOURCES.get_or_init(Resource::new).captures)
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
