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

//! Packet Capture handlers and singleton for HTTP and gRPC server.
//!
//! This module implements a handler for GET, PATCH, LIST pcap
//!
//! /v1/captures --> handle_capture_list
//! /v1/captures/{id} --> handle_capture_patch, handle_capture_get
//! handle_pcap_cxx calls handle_capture, which calls handle_capture_* based on uri
//! handle_packet_request and handle_packet_response is invoked by packet_hub
//! to write packets to files if capture state is on.

use cxx::CxxVector;
use frontend_proto::common::ChipKind;
use frontend_proto::frontend::{GetDevicesResponse, GetPcapResponse};
use lazy_static::lazy_static;
use protobuf::Message;
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ffi::{get_devices_bytes, CxxServerResponseWriter};
use crate::http_server::http_request::{HttpHeaders, HttpRequest};
use crate::http_server::server_response::ResponseWritable;
use crate::pcap::capture::{Captures, ChipId};
use crate::CxxServerResponseWriterWrapper;

use super::capture::CaptureInfo;
use super::pcap_util::{append_record, PacketDirection};
use super::proto_json::capture_to_string;

// The Capture resource is a singleton that manages all captures
lazy_static! {
    static ref RESOURCE: RwLock<Captures> = RwLock::new(Captures::new());
}

// Update the Captures collection to reflect the currently connected devices.
// This function removes entries from Pcaps when devices/chips
// go away and adds entries when new devices/chips connect.
//
// Note: if a device disconnects and there is captured data, the entry
// remains with a flag valid = false so it can be retrieved.
fn update_captures(captures: &mut Captures) {
    // Perform get_devices_bytes ffi to receive bytes of GetDevicesResponse
    // Print error and return empty hashmap if GetDevicesBytes fails.
    let mut vec = Vec::<u8>::new();
    if !get_devices_bytes(&mut vec) {
        println!("netsim error: GetDevicesBytes failed - returning an empty set of pcaps");
        return;
    }

    // Parse get_devices_response
    let device_response = GetDevicesResponse::parse_from_bytes(&vec).unwrap();

    // Adding to Captures hashmap
    let mut chip_ids = HashSet::<ChipId>::new();
    for device in device_response.get_devices() {
        for chip in device.get_chips() {
            chip_ids.insert(chip.get_id());
            if !captures.contains(chip.get_id()) {
                let capture =
                    CaptureInfo::new(chip.get_kind(), chip.get_id(), device.get_name().into());
                captures.insert(capture);
            }
        }
    }

    // Two cases when device gets disconnected:
    // 1. The device had no capture, remove completely.
    // 2. The device had capture, indicate by capture.set_valid(false)
    enum RemovalIndicator {
        Gone(ChipId),   // type ChipId = i32
        Unused(ChipId), // type ChipId = i32
    }

    // Check if the active_pcap entry still exists in the chips.
    let mut removal = Vec::<RemovalIndicator>::new();
    for (chip_id, capture) in captures.iter() {
        let lock = capture.lock().unwrap();
        let proto_capture = lock.get_capture_proto();
        if !chip_ids.contains(chip_id) {
            if proto_capture.get_size() == 0 {
                removal.push(RemovalIndicator::Unused(chip_id.to_owned()));
            } else {
                removal.push(RemovalIndicator::Gone(chip_id.to_owned()))
            }
        }
    }

    // Now remove/update the pcaps based on the loop above
    for indicator in removal {
        match indicator {
            RemovalIndicator::Unused(key) => captures.remove(&key),
            RemovalIndicator::Gone(key) => {
                for capture in captures.get(key).iter() {
                    capture.lock().unwrap().valid = false;
                }
            }
        }
    }
}

pub fn handle_capture_list(writer: ResponseWritable, captures: &mut Captures) {
    // Get the most updated active captures
    update_captures(captures);

    // Write active captures to json string (will be deprecated with protobuf v3)
    let mut out = String::new();
    if captures.is_empty() {
        out.push_str(r#"{}"#);
    } else {
        out.push_str(r#"{"pcaps": ["#);
        for capture in captures.values() {
            capture_to_string(&capture.lock().unwrap().get_capture_proto(), &mut out)
        }
        out.pop();
        out.push_str(r"]}");
    }
    writer.put_ok("text/json", out.as_str());
}

pub fn handle_capture_patch(
    writer: ResponseWritable,
    captures: &mut Captures,
    id: ChipId,
    state: bool,
) {
    // Get the most updated active captures
    update_captures(captures);

    if let Some(mut capture) = captures.get(id).map(|arc_capture| arc_capture.lock().unwrap()) {
        match state {
            true => {
                if let Err(err) = capture.start_capture() {
                    writer.put_error(404, err.to_string().as_str());
                    return;
                }
            }
            false => capture.stop_capture(),
        }

        // Write result to writer
        let proto_capture = capture.get_capture_proto();
        let mut out = String::new();
        capture_to_string(&proto_capture, &mut out);
        out.pop();
        writer.put_ok("text/json", out.as_str());
    }
}

/// The Rust capture handler used directly by Http frontend for LIST, GET, and PATCH
pub fn handle_capture(request: &HttpRequest, param: &str, writer: ResponseWritable) {
    if request.uri.as_str() == "/v1/captures" {
        match request.method.as_str() {
            "GET" => {
                let mut captures = RESOURCE.write().unwrap();
                handle_capture_list(writer, &mut captures);
            }
            _ => writer.put_error(404, "Not found."),
        }
    } else {
        match request.method.as_str() {
            "GET" => {
                // TODO: Implement handle_capture_get in controller.rs
                writer.put_ok_with_length("text/plain", 0);
                let response_bytes = GetPcapResponse::new().write_to_bytes().unwrap();
                writer.put_chunk(&response_bytes);
                writer.put_chunk(&response_bytes);
            }
            "PATCH" => {
                let mut captures = RESOURCE.write().unwrap();
                let id = match param.parse::<i32>() {
                    Ok(num) => num,
                    Err(_) => {
                        writer.put_error(404, "Incorrect ID type for pcap, ID should be i32.");
                        return;
                    }
                };
                let body = &request.body;
                let state = String::from_utf8(body.to_vec()).unwrap();
                match state.as_str() {
                    "1" => handle_capture_patch(writer, &mut captures, id, true),
                    "2" => handle_capture_patch(writer, &mut captures, id, false),
                    _ => writer.put_error(404, "Incorrect state for PatchPcap"),
                }
            }
            _ => writer.put_error(404, "Not found."),
        }
    }
}

/// pcap handle cxx for grpc server to call
pub fn handle_pcap_cxx(
    responder: Pin<&mut CxxServerResponseWriter>,
    method: String,
    param: String,
    body: String,
) {
    let mut request = HttpRequest {
        method,
        uri: String::new(),
        headers: HttpHeaders::new(),
        version: "1.1".to_string(),
        body: body.as_bytes().to_vec(),
    };
    if param.is_empty() {
        request.uri = "/v1/captures".to_string();
    } else {
        request.uri = format!("/v1/captures/{}", param);
    }
    handle_capture(
        &request,
        param.as_str(),
        &mut CxxServerResponseWriterWrapper { writer: responder },
    );
}

// Helper function for translating u32 representation of ChipKind
fn int_to_chip_kind(kind: u32) -> ChipKind {
    match kind {
        1 => ChipKind::BLUETOOTH,
        2 => ChipKind::WIFI,
        3 => ChipKind::UWB,
        _ => ChipKind::UNSPECIFIED,
    }
}

// A common code for handle_request and handle_response cxx mehtods
fn handle_packet(
    kind: u32,
    facade_id: u32,
    packet: &CxxVector<u8>,
    packet_type: u32,
    direction: PacketDirection,
) {
    let pcaps = RESOURCE.read().unwrap();
    let facade_key = CaptureInfo::new_facade_key(int_to_chip_kind(kind), facade_id as i32);
    if let Some(mut pcap) =
        pcaps.facade_key_to_capture.get(&facade_key).map(|arc_pcap| arc_pcap.lock().unwrap())
    {
        if let Some(ref mut file) = pcap.file {
            if int_to_chip_kind(kind) == ChipKind::BLUETOOTH {
                let timestamp =
                    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
                match append_record(timestamp, file, direction, packet_type, packet.as_slice()) {
                    Ok(size) => {
                        pcap.size += size;
                        pcap.records += 1;
                    }
                    Err(err) => {
                        println!("netsimd: {err:?}");
                    }
                }
            }
        }
    };
}

// Cxx Method for packet_hub to invoke (Host to Controller Packet Flow)
pub fn handle_packet_request(kind: u32, facade_id: u32, packet: &CxxVector<u8>, packet_type: u32) {
    handle_packet(kind, facade_id, packet, packet_type, PacketDirection::HostToController)
}

// Cxx Method for packet_hub to invoke (Controller to Host Packet Flow)
pub fn handle_packet_response(kind: u32, facade_id: u32, packet: &CxxVector<u8>, packet_type: u32) {
    handle_packet(kind, facade_id, packet, packet_type, PacketDirection::ControllerToHost)
}
