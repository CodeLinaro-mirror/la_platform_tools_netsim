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

use crate::bluetooth as bluetooth_facade;
use crate::config::get_dev;
use netsim_common::util::netsim_logger;

/// Module to control startup, run, and cleanup netsimd services.
// TODO: Replace Run() in server.cc.

pub struct Service {
    // netsimd states, like device resource.
}

impl Service {
    pub fn new() -> Service {
        Service {}
    }

    /// Sets up the states for netsimd.
    pub fn set_up(&self) {
        // TODO: clean pcap files.
        netsim_logger::init("netsimd");
    }

    /// Runs the netsimd services.
    pub fn run(&self) {
        // TODO: run servers, like calling run_http_server().

        if get_dev() {
            // Create two beacon devices in dev mode.
            bluetooth_facade::beacon::new_beacon(
                "test_beacon1".to_string(),
                "be:ac:01:55:00:01".to_string(),
            );
            bluetooth_facade::beacon::new_beacon(
                "test_beacon2".to_string(),
                "be:ac:01:55:00:02".to_string(),
            );
        }
    }
}

// For cxx.
pub fn create_service() -> Box<Service> {
    Box::new(Service {})
}
