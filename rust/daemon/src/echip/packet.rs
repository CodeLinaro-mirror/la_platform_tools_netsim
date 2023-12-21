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

use std::collections::HashMap;
use std::sync::{
    mpsc::{channel, Sender},
    Arc, Mutex,
};
use std::thread;

use crate::captures::captures_handler;
use crate::devices::chip::ChipIdentifier;
use crate::echip::get;

use lazy_static::lazy_static;
use log::{error, info, warn};
use netsim_proto::hci_packet::hcipacket::PacketType;
use protobuf::Enum;

/// The Packet module routes packets from a chip controller instance to
/// different transport managers. Currently transport managers include
///
/// - GRPC is a PacketStreamer
/// - FD is a file descriptor to a pair of Unix Fifos used by "-s" startup
/// - SOCKET is a TCP stream

// When a connection arrives, the transport registers a responder
// implementing Response trait for the packet stream.
pub trait Response {
    fn response(&mut self, packet: Vec<u8>, packet_type: u8);
}

// When a responder is registered a responder thread is created to
// decouple the chip controller from the network. The thread reads
// ResponsePacket from a queue and sends to responder.
struct ResponsePacket {
    packet: Vec<u8>,
    packet_type: u8,
}

// SENDERS is a singleton that contains a hash map from
// (kind,facade_id) to responder queue.

lazy_static! {
    static ref SENDERS: Arc<Mutex<HashMap<String, Sender<ResponsePacket>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

fn get_key(kind: u32, facade_id: u32) -> String {
    format!("{}/{}", kind, facade_id)
}

/// Register a chip controller instance to a transport manager.
pub fn register_transport(kind: u32, facade_id: u32, mut responder: Box<dyn Response + Send>) {
    let key = get_key(kind, facade_id);
    let (tx, rx) = channel::<ResponsePacket>();
    match SENDERS.lock() {
        Ok(mut map) => {
            if map.contains_key(&key) {
                error!("register_transport: key already present for chip_kind/facade_id: {key}");
            }
            map.insert(key.clone(), tx);
        }
        Err(_) => panic!("register_transport: poisoned lock"),
    }
    let _ = thread::Builder::new().name("transport_writer_{key}".to_string()).spawn(move || {
        info!("register_transport: started thread chip_kind/facade_id: {key}");
        loop {
            match rx.recv() {
                Ok(ResponsePacket { packet, packet_type }) => {
                    responder.response(packet, packet_type);
                }
                Err(_) => {
                    info!("register_transport: finished thread chip_kind/facade_id: {key}");
                    break;
                }
            }
        }
    });
}

/// Unregister a chip controller instance.
pub fn unregister_transport(kind: u32, facade_id: u32) {
    let key = get_key(kind, facade_id);
    // Shuts down the responder thread, because sender is dropped.
    SENDERS.lock().expect("unregister_transport: poisoned lock").remove(&key);
}

// Handle response from facades.
//
// Queue the response packet to be handled by the responder thread.
//
pub fn handle_response(kind: u32, facade_id: u32, packet: &cxx::CxxVector<u8>, packet_type: u8) {
    // TODO(b/314840701):
    // 1. Per EChip Struct should contain private field of channel & facade_id
    // 2. Lookup from ECHIPS with given chip_id
    // 3. Call echips.handle_response
    let packet_vec = packet.as_slice().to_vec();
    captures_handler::handle_packet_response(kind, facade_id, &packet_vec, packet_type.into());

    let key = get_key(kind, facade_id);
    let mut binding = SENDERS.lock().expect("Failed to acquire lock on SENDERS");
    if let Some(responder) = binding.get(&key) {
        if responder.send(ResponsePacket { packet: packet_vec, packet_type }).is_err() {
            warn!("handle_response: send failed for chip_kind/facade_id: {key}");
            binding.remove(&key);
        }
    } else {
        warn!("handle_response: unknown chip_kind: {kind} facade_id: {facade_id}");
    };
}

/// Handle requests from transports.
pub fn handle_request(
    kind: u32,
    facade_id: u32,
    chip_id: ChipIdentifier,
    packet: &mut Vec<u8>,
    packet_type: u8,
) {
    captures_handler::handle_packet_request(kind, facade_id, packet, packet_type.into());

    // Prepend packet_type to packet if specified
    if PacketType::HCI_PACKET_UNSPECIFIED.value()
        != <u8 as std::convert::Into<i32>>::into(packet_type)
    {
        packet.insert(0, packet_type);
    }

    // Perform handle_request
    match get(chip_id) {
        Some(emulated_chip) => emulated_chip.handle_request(packet),
        None => warn!("SharedEmulatedChip doesn't exist for {chip_id}"),
    };
}

/// Handle requests from transports in C++.
pub fn handle_request_cxx(
    kind: u32,
    facade_id: u32,
    chip_id: u32,
    packet: &cxx::CxxVector<u8>,
    packet_type: u8,
) {
    let mut packet_vec = packet.as_slice().to_vec();
    handle_request(kind, facade_id, chip_id, &mut packet_vec, packet_type);
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
        fn response(&mut self, _packet: Vec<u8>, _packet_type: u8) {}
    }

    #[test]
    fn test_register_transport() {
        let val: Box<dyn Response + Send> = Box::new(TestTransport {});
        register_transport(0, 0, val);
        let key = get_key(0, 0);
        {
            let binding = SENDERS.lock().unwrap();
            assert!(binding.contains_key(&key));
        }

        SENDERS.lock().unwrap().remove(&key);
    }

    #[test]
    fn test_unregister_transport() {
        register_transport(0, 1, Box::new(TestTransport {}));
        unregister_transport(0, 1);
        assert!(SENDERS.lock().unwrap().get(&get_key(0, 1)).is_none());
    }
}
