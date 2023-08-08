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

use std::sync::{Arc, Mutex};
use std::{collections::HashMap, io::Cursor, net::TcpStream};

use frontend_proto::common::ChipKind;
use log::{error, info};
use tungstenite::{protocol::Role, Message, WebSocket};

use crate::http_server::{
    collect_query, http_request::HttpRequest, server_response::ResponseWritable,
};
use crate::{
    devices::devices_handler::{add_chip, remove_chip},
    ffi::handle_request_cxx,
    transport::{dispatcher::unregister_transport, h4},
};

use super::dispatcher::{register_transport, Response};

// This feature is enabled only for CMake builds
#[cfg(feature = "local_ssl")]
use crate::openssl;

/// Generate Sec-Websocket-Accept value from given Sec-Websocket-Key value
fn generate_websocket_accept(websocket_key: String) -> String {
    let concat = websocket_key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let hashed = openssl::sha::sha1(concat.as_bytes());
    data_encoding::BASE64.encode(&hashed)
}

/// Check if all required fields exist in queries and request headers
fn has_required_websocket_fields(queries: &HashMap<&str, &str>, request: &HttpRequest) -> bool {
    queries.contains_key("name")
        && queries.contains_key("kind")
        && request.headers.get("Sec-Websocket-Key").is_some()
}

/// Handler for websocket server connection
pub fn handle_websocket(request: &HttpRequest, param: &str, writer: ResponseWritable) {
    match collect_query(param) {
        Ok(queries) if has_required_websocket_fields(&queries, request) => {
            let websocket_accept =
                generate_websocket_accept(request.headers.get("Sec-Websocket-Key").unwrap());
            writer.put_ok_switch_protocol(
                "websocket",
                &[("Sec-WebSocket-Accept", websocket_accept.as_str())],
            )
        }
        Ok(_) => writer.put_error(404, "Missing query fields and/or Sec-Websocket-Key"),
        Err(err) => writer.put_error(404, err),
    }
}

struct WebSocketTransport {
    websocket_writer: Arc<Mutex<WebSocket<TcpStream>>>,
}

impl Response for WebSocketTransport {
    fn response(&mut self, packet: &cxx::CxxVector<u8>, packet_type: u8) {
        let mut buffer = Vec::new();
        buffer.push(packet_type);
        buffer.extend(packet);
        if let Err(err) =
            self.websocket_writer.lock().unwrap().write_message(Message::Binary(buffer))
        {
            error!("{err}");
        };
    }
}

/// Run websocket transport for packet flow in netsim
pub fn run_websocket_transport(stream: TcpStream, queries: HashMap<&str, &str>) {
    // Add Chip
    let result = match add_chip(
        &stream.peer_addr().unwrap().port().to_string(),
        queries.get("name").unwrap(),
        ChipKind::BLUETOOTH,
        &format!("websocket-{}", stream.peer_addr().unwrap()),
        "Google",
        "Google",
    ) {
        Ok(chip_result) => chip_result,
        Err(err) => {
            error!("{err}");
            return;
        }
    };

    // Create websocket_writer to handle packet responses, write pong or close messages
    let websocket_writer = Arc::new(Mutex::new(WebSocket::from_raw_socket(
        stream.try_clone().unwrap(),
        Role::Server,
        None,
    )));
    // Websocket reader
    let mut websocket_reader = WebSocket::from_raw_socket(stream, Role::Server, None);

    // Sending cloned websocket into packet dispatcher
    register_transport(
        ChipKind::BLUETOOTH as u32,
        result.facade_id,
        Box::new(WebSocketTransport { websocket_writer: websocket_writer.clone() }),
    );

    // Running Websocket server
    loop {
        let packet_msg =
            match websocket_reader.read_message().map_err(|_| "Failed to read Websocket message") {
                Ok(message) => message,
                Err(err) => {
                    error!("{err}");
                    break;
                }
            };
        if packet_msg.is_binary() {
            let mut cursor = Cursor::new(packet_msg.into_data());
            match h4::read_h4_packet(&mut cursor) {
                Ok(packet) => {
                    let kind = ChipKind::BLUETOOTH as u32;
                    handle_request_cxx(kind, result.facade_id, &packet.payload, packet.h4_type);
                }
                Err(error) => {
                    error!(
                        "netsimd: end websocket reader {}: {:?}",
                        websocket_reader.get_ref().peer_addr().unwrap(),
                        error
                    );
                    break;
                }
            }
        } else if packet_msg.is_ping() {
            if let Err(err) = websocket_writer
                .lock()
                .unwrap()
                .write_message(Message::Pong(packet_msg.into_data()))
            {
                error!("{err}");
            }
        } else if packet_msg.is_close() {
            if let Message::Close(close_frame) = packet_msg {
                if let Err(err) = websocket_writer
                    .lock()
                    .unwrap()
                    .close(close_frame)
                    .map_err(|_| "Failed to close Websocket")
                {
                    error!("{err}");
                }
            }
            break;
        }
    }

    // Remove Chip
    info!("remove chip: device {}, chip {}", result.device_id, result.chip_id);
    if let Err(err) = remove_chip(result.device_id, result.chip_id) {
        error!("{err}");
    };
    // The connection will be closed when the value is dropped.
    unregister_transport(ChipKind::BLUETOOTH as u32, result.facade_id);
}
