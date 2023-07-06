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
use log::warn;
use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

/// The Dispatcher module routes packets from a chip controller instance to
/// different transport managers. Currently transport managers include
///
/// - GRPC is a PacketStreamer
/// - FD is a file descriptor to a pair of Unix Fifos used by "-s" startup
/// - SOCKET is a TCP stream

pub trait Response {
    fn response(&mut self, packet: &cxx::CxxVector<u8>, packet_type: u8);
}

// TRANSPORTS is a singleton that contains a hash map from (kind,facade id) to Response.
lazy_static! {
    static ref TRANSPORTS: RwLock<HashMap<String, Mutex<Box<dyn Response + Send>>>> =
        RwLock::new(HashMap::new());
}

fn get_key(kind: u32, facade_id: u32) -> String {
    format!("{}/{}", kind, facade_id)
}

/// Register a chip controller instance to a transport manager.
pub fn register_transport(kind: u32, facade_id: u32, response: Box<dyn Response + Send>) {
    let key = get_key(kind, facade_id);
    TRANSPORTS.write().unwrap().insert(key, Mutex::new(response));
}

/// Unregister a chip controller instance.
pub fn unregister_transport(kind: u32, facade_id: u32) {
    let key = get_key(kind, facade_id);
    TRANSPORTS.write().unwrap().remove(&key);
}

/// For packet_hub in C++.
pub fn handle_response(kind: u32, facade_id: u32, packet: &cxx::CxxVector<u8>, packet_type: u8) {
    let binding = TRANSPORTS.read().unwrap();
    if let Some(response) = binding.get(&get_key(kind, facade_id)) {
        response.lock().unwrap().response(packet, packet_type);
    } else {
        warn!("Failed to dispatch response for unknown chip kind `{kind}` and facade ID `{facade_id}`.");
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_key() {
        assert_eq!("0/0", get_key(0, 0));
        assert_eq!("42/21", get_key(42, 21));
        assert_eq!("666/1234", get_key(666, 1234));
    }

    struct TestTransport {}
    impl Response for TestTransport {
        fn response(&mut self, _packet: &cxx::CxxVector<u8>, _packet_type: u8) {}
    }

    #[test]
    fn test_register_transport() {
        let val: Box<dyn Response + Send> = Box::new(TestTransport {});
        register_transport(0, 0, val);
        let key = get_key(0, 0);
        {
            let binding = TRANSPORTS.read().unwrap();
            assert!(binding.contains_key(&key));
        }

        TRANSPORTS.write().unwrap().remove(&key);
    }

    #[test]
    fn test_unregister_transport() {
        register_transport(0, 1, Box::new(TestTransport {}));
        unregister_transport(0, 1);
        assert!(TRANSPORTS.read().unwrap().get(&get_key(0, 1)).is_none());
    }
}
