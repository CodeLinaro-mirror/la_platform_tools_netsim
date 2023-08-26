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
//! This module implements a handler for GET, PATCH, LIST capture
//!
//! /v1/captures --> handle_capture_list
//!
//! /v1/captures/{id} --> handle_capture_patch, handle_capture_get
//!
//! handle_capture_cxx calls handle_capture, which calls handle_capture_* based on uri.
//! handle_packet_request and handle_packet_response is invoked by packet_hub
//! to write packets to files if capture state is on.

// TODO(b/274506882): Implement gRPC status proto on error responses. Also write better
// and more descriptive error messages with proper error codes.

use http::{Request, Version};
use log::warn;
use netsim_common::util::time_display::TimeDisplay;
use netsim_proto::common::ChipKind;
use netsim_proto::frontend::ListCaptureResponse;
use protobuf_json_mapping::{print_to_string_with_options, PrintOptions};
use std::fs::File;
use std::io::{Read, Result};
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::devices::chip::ChipIdentifier;
use crate::ffi::CxxServerResponseWriter;
use crate::ffi::CxxServerResponseWriterWrapper;
use crate::http_server::server_response::ResponseWritable;
use crate::resource::clone_captures;
use crate::util::int_to_chip_kind;

use super::capture::CaptureInfo;
use super::pcap_util::{append_record, PacketDirection};
use super::PCAP_MIME_TYPE;

const CHUNK_LEN: usize = 1024;
const JSON_PRINT_OPTION: PrintOptions = PrintOptions {
    enum_values_int: false,
    proto_field_name: false,
    always_output_default_values: true,
    _future_options: (),
};

/// Helper function for getting file name from the given fields.
fn get_file(id: ChipIdentifier, device_name: String, chip_kind: ChipKind) -> Result<File> {
    let mut filename = netsim_common::system::netsimd_temp_dir();
    filename.push("pcaps");
    filename.push(format!("{:?}-{:}-{:?}.pcap", id, device_name, chip_kind));
    File::open(filename)
}

// TODO: GetCapture should return the information of the capture. Need to reconsider
// uri hierarchy.
// GET /captures/id/{id} --> Get Capture information
// GET /captures/contents/{id} --> Download Pcap file
/// Performs GetCapture to download pcap file and write to writer.
pub fn handle_capture_get(writer: ResponseWritable, id: ChipIdentifier) {
    let captures_arc = clone_captures();
    let mut captures = captures_arc.write().unwrap();
    if let Some(capture) = captures.get(id).map(|arc_capture| arc_capture.lock().unwrap()) {
        if capture.size == 0 {
            writer.put_error(
                404,
                &format!(
                    "Capture file not found for {:?}-{}-{:?}",
                    id, capture.device_name, capture.chip_kind
                ),
            );
        } else if let Ok(mut file) = get_file(id, capture.device_name.clone(), capture.chip_kind) {
            let mut buffer = [0u8; CHUNK_LEN];
            let time_display = TimeDisplay::new(capture.seconds, capture.nanos as u32);
            let header_value = format!(
                "attachment; filename=\"{:?}-{:}-{:?}-{}.pcap\"",
                id,
                capture.device_name.clone(),
                capture.chip_kind,
                time_display.utc_display()
            );
            writer.put_ok_with_length(
                PCAP_MIME_TYPE,
                capture.size,
                vec![("Content-Disposition".to_string(), header_value)],
            );
            loop {
                match file.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(length) => writer.put_chunk(&buffer[..length]),
                    Err(_) => {
                        writer.put_error(404, "Error reading pcap file");
                        break;
                    }
                }
            }
        } else {
            writer.put_error(404, "Cannot open Capture file");
        }
    } else {
        writer.put_error(404, "Cannot access Capture Resource")
    };
}

/// Performs ListCapture to get the list of CaptureInfos and write to writer.
pub fn handle_capture_list(writer: ResponseWritable) {
    let captures_arc = clone_captures();
    let captures = captures_arc.write().unwrap();
    // Instantiate ListCaptureResponse and add Captures
    let mut response = ListCaptureResponse::new();
    for capture in captures.values() {
        response.captures.push(capture.lock().unwrap().get_capture_proto());
    }

    // Perform protobuf-json-mapping with the given protobuf
    if let Ok(json_response) = print_to_string_with_options(&response, &JSON_PRINT_OPTION) {
        writer.put_ok("text/json", &json_response, vec![])
    } else {
        writer.put_error(404, "proto to JSON mapping failure")
    }
}

/// Performs PatchCapture to patch a CaptureInfo with id.
/// Writes the result of PatchCapture to writer.
pub fn handle_capture_patch(writer: ResponseWritable, id: ChipIdentifier, state: bool) {
    let captures_arc = clone_captures();
    let mut captures = captures_arc.write().unwrap();
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

        // Perform protobuf-json-mapping with the given protobuf
        if let Ok(json_response) =
            print_to_string_with_options(&capture.get_capture_proto(), &JSON_PRINT_OPTION)
        {
            writer.put_ok("text/json", &json_response, vec![]);
        } else {
            writer.put_error(404, "proto to JSON mapping failure");
        }
    };
}

/// The Rust capture handler used directly by Http frontend or handle_capture_cxx for LIST, GET, and PATCH
pub fn handle_capture(request: &Request<Vec<u8>>, param: &str, writer: ResponseWritable) {
    if request.uri() == "/v1/captures" {
        match request.method().as_str() {
            "GET" => {
                handle_capture_list(writer);
            }
            _ => writer.put_error(404, "Not found."),
        }
    } else {
        match request.method().as_str() {
            "GET" => {
                let id = match param.parse::<u32>() {
                    Ok(num) => num,
                    Err(_) => {
                        writer.put_error(404, "Incorrect ID type for capture, ID should be u32.");
                        return;
                    }
                };
                handle_capture_get(writer, id);
            }
            "PATCH" => {
                let id = match param.parse::<u32>() {
                    Ok(num) => num,
                    Err(_) => {
                        writer.put_error(404, "Incorrect ID type for capture, ID should be u32.");
                        return;
                    }
                };
                let body = request.body();
                let state = String::from_utf8(body.to_vec()).unwrap();
                match state.as_str() {
                    "1" => handle_capture_patch(writer, id, true),
                    "2" => handle_capture_patch(writer, id, false),
                    _ => writer.put_error(404, "Incorrect state for PatchCapture"),
                }
            }
            _ => writer.put_error(404, "Not found."),
        }
    }
}

/// Capture handler cxx for grpc server to call
pub fn handle_capture_cxx(
    responder: Pin<&mut CxxServerResponseWriter>,
    method: String,
    param: String,
    body: String,
) {
    let mut builder = Request::builder().method(method.as_str());
    if param.is_empty() {
        builder = builder.uri("/v1/captures");
    } else {
        builder = builder.uri(format!("/v1/captures/{}", param));
    }
    builder = builder.version(Version::HTTP_11);
    let request = match builder.body(body.as_bytes().to_vec()) {
        Ok(request) => request,
        Err(err) => {
            warn!("{err:?}");
            return;
        }
    };
    handle_capture(
        &request,
        param.as_str(),
        &mut CxxServerResponseWriterWrapper { writer: responder },
    );
}

/// A common code for handle_request and handle_response methods.
fn handle_packet(
    kind: u32,
    facade_id: u32,
    packet: &Vec<u8>,
    packet_type: u32,
    direction: PacketDirection,
) {
    let captures_arc = clone_captures();
    let captures = captures_arc.write().unwrap();
    let facade_key = CaptureInfo::new_facade_key(int_to_chip_kind(kind), facade_id);
    if let Some(mut capture) = captures
        .facade_key_to_capture
        .get(&facade_key)
        .map(|arc_capture| arc_capture.lock().unwrap())
    {
        if let Some(ref mut file) = capture.file {
            if int_to_chip_kind(kind) == ChipKind::BLUETOOTH {
                let timestamp =
                    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
                match append_record(timestamp, file, direction, packet_type, packet.as_slice()) {
                    Ok(size) => {
                        capture.size += size;
                        capture.records += 1;
                    }
                    Err(err) => {
                        warn!("{err:?}");
                    }
                }
            }
        }
    };
}

/// Method for dispatcher to invoke (Host to Controller Packet Flow)
pub fn handle_packet_request(kind: u32, facade_id: u32, packet: &Vec<u8>, packet_type: u32) {
    handle_packet(kind, facade_id, packet, packet_type, PacketDirection::HostToController)
}

/// Method for dispatcher to invoke (Controller to Host Packet Flow)
pub fn handle_packet_response(kind: u32, facade_id: u32, packet: &Vec<u8>, packet_type: u32) {
    handle_packet(kind, facade_id, packet, packet_type, PacketDirection::ControllerToHost)
}

/// Method for clearing pcap files in temp directory
pub fn clear_pcap_files() -> bool {
    let mut path = netsim_common::system::netsimd_temp_dir();
    path.push("pcaps");

    // Check if the directory exists.
    if std::fs::metadata(&path).is_err() {
        return false;
    }

    // Delete the directory.
    std::fs::remove_dir_all(&path).is_ok()
}
