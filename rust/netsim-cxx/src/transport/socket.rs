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

use super::dispatcher::{register_transport, unregister_transport, Response};
use crate::ffi::{add_chip_cxx, handle_request_cxx, remove_chip};
use crate::transport::h4;
use cxx::let_cxx_string;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::thread;

// The HCI server implements the Bluetooth UART transport protocol
// (a.k.a. H4) over TCP. Each new connection on the HCI port spawns a
// new virtual controller associated with a new device.

/// Start the socket-based transport.
///
/// The socket transport reads/writes host-controller messages
/// for bluetooth (h4 hci) over a [TcpStream] transport.
///

struct SocketTransport {
    stream: TcpStream,
}

impl Response for SocketTransport {
    fn response(&mut self, packet: &cxx::CxxVector<u8>, packet_type: u8) {
        let mut buffer = Vec::new();
        buffer.push(packet_type);
        buffer.extend(packet);
        if let Err(e) = self.stream.write_all(&buffer[..]) {
            println!("netsimd: error writing {}", e);
        };
    }
}

pub fn run_socket_transport(hci_port: u16) {
    thread::Builder::new()
        .name("hci_transport".to_string())
        .spawn(move || {
            accept_incoming(hci_port)
                .unwrap_or_else(|e| println!("netsimd: Failed to accept incoming stream: {:?}", e));
        })
        .unwrap();
}

fn accept_incoming(hci_port: u16) -> std::io::Result<()> {
    let hci_socket = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, hci_port);
    let listener = TcpListener::bind(hci_socket)?;
    println!("netsimd: hci listening on: {}", hci_port);

    for stream in listener.incoming() {
        let stream = stream?;
        println!("netsimd: hci_client host addr: {}", stream.peer_addr().unwrap());
        thread::Builder::new()
            .name("hci_transport client".to_string())
            .spawn(move || {
                handle_hci_client(stream);
            })
            .unwrap();
    }
    Ok(())
}

#[derive(Copy, Clone)]
enum ChipKind {
    UNSPECIFIED = 0,
    BLUETOOTH = 1,
    WIFI = 2,
    UWB = 3,
}

fn handle_hci_client(stream: TcpStream) {
    // ...
    let_cxx_string!(guid = stream.peer_addr().unwrap().port().to_string());
    let_cxx_string!(device_name = format!("socket-{}", stream.peer_addr().unwrap()));
    let_cxx_string!(name = format!("socket-{}", stream.peer_addr().unwrap()));
    let_cxx_string!(manufacturer = "Google");
    let_cxx_string!(product_name = "Google");
    let result = add_chip_cxx(
        &guid,
        &device_name,
        ChipKind::BLUETOOTH as u32,
        &name,
        &manufacturer,
        &product_name,
    );
    let tcp_rx = stream.try_clone().unwrap();
    register_transport(
        ChipKind::BLUETOOTH as u32,
        result.get_facade_id(),
        Box::new(SocketTransport { stream }),
    );

    let _ = reader(tcp_rx, ChipKind::BLUETOOTH, result.get_facade_id());

    println!(
        "netsimd: remove chip: device {}, chip {}",
        result.get_device_id(),
        result.get_chip_id()
    );
    remove_chip(result.get_device_id(), result.get_chip_id());
    // The connection will be closed when the value is dropped.
    unregister_transport(ChipKind::BLUETOOTH as u32, result.get_facade_id());
}

/// read from the socket and pass to the packet hub.
///
fn reader(mut tcp_rx: TcpStream, kind: ChipKind, facade_id: u32) -> std::io::Result<()> {
    loop {
        if let ChipKind::BLUETOOTH = kind {
            match h4::read_h4_packet(&mut tcp_rx) {
                Ok(packet) => {
                    let kind: u32 = kind as u32;
                    handle_request_cxx(kind, facade_id, &packet.payload, packet.h4_type);
                }
                Err(error) => {
                    println!(
                        "netsimd: end socket reader {}: {:?}",
                        &tcp_rx.peer_addr().unwrap(),
                        error
                    );
                    return Ok(());
                }
            }
        }
    }
}
