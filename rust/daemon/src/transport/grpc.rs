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

use crate::echip::packet::{register_transport, unregister_transport, Response};
use crate::ffi::ffi_transport::handle_grpc_response;

/// Grpc transport.
///
/// This module provides a wrapper around the C++ Grpc implementation. It
/// provides a higher-level API that is easier to use from Rust.

struct GrpcTransport {
    kind: u32,
    facade_id: u32,
}

impl Response for GrpcTransport {
    fn response(&mut self, packet: Vec<u8>, packet_type: u8) {
        handle_grpc_response(self.kind, self.facade_id, &packet, packet_type)
    }
}

// for grpc server in C++
pub fn register_grpc_transport(kind: u32, facade_id: u32) {
    register_transport(kind, facade_id, Box::new(GrpcTransport { kind, facade_id }));
}

// for grpc server in C++
pub fn unregister_grpc_transport(kind: u32, facade_id: u32) {
    unregister_transport(kind, facade_id);
}
