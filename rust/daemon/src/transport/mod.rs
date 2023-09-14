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

pub mod dispatcher;
#[cfg(feature = "cuttlefish")]
pub mod fd;
pub mod grpc;
mod h4;
pub mod socket;
#[cfg(feature = "cuttlefish")]
mod uci;
pub mod websocket;

// This provides no-op implementations of fd transport for non-unix systems.
#[cfg(not(feature = "cuttlefish"))]
pub mod fd {
    pub fn handle_response(
        _kind: u32,
        _facade_id: u32,
        _packet: &cxx::CxxVector<u8>,
        _packet_type: u8,
    ) {
    }
    #[allow(clippy::ptr_arg)]
    pub fn run_fd_transport(_startup_json: &String) {}
}
