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

//! Netsim cxx libraries.

mod frontend_http_server;
mod ranging;
mod version;

use crate::frontend_http_server::run_frontend_http_server;
use crate::ranging::*;
use crate::version::*;

#[cxx::bridge(namespace = "netsim")]
mod ffi {

    extern "Rust" {

        #[cxx_name = "RunFrontendHttpServer"]
        fn run_frontend_http_server();

        // Ranging

        #[cxx_name = "DistanceToRssi"]
        fn distance_to_rssi(tx_power: i8, distance: f32) -> i8;

        // Version

        #[cxx_name = "GetVersion"]
        fn get_version() -> String;
    }

    unsafe extern "C++" {
        include!("controller/controller.h");

        #[allow(dead_code)]
        #[rust_name = "get_devices"]
        #[namespace = "netsim::scene_controller"]
        fn GetDevices(
            request: &CxxString,
            response: Pin<&mut CxxString>,
            error_message: Pin<&mut CxxString>,
        ) -> u32;

        #[allow(dead_code)]
        #[rust_name = "patch_device"]
        #[namespace = "netsim::scene_controller"]
        fn PatchDevice(
            request: &CxxString,
            response: Pin<&mut CxxString>,
            error_message: Pin<&mut CxxString>,
        ) -> u32;
    }
}
