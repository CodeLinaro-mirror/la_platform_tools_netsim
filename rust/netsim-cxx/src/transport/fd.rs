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
/// request packets flow into netsim
/// response packets flow out of netsim
/// packet transports read requests and write response packets over gRPC or Fds.
use super::h4;
use super::h4::PacketError;
use super::uci;
use crate::devices::devices_handler::{add_chip, remove_chip};
use crate::ffi::handle_request_cxx;
use frontend_proto::common::ChipKind;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::os::fd::FromRawFd;
use std::thread::JoinHandle;
use std::{fmt, thread};

#[derive(Serialize, Deserialize, Debug)]
struct StartupInfo {
    devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Device {
    name: String,
    chips: Vec<Chip>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum ChipKindEnum {
    UNKNOWN = 0,
    BLUETOOTH = 1,
    WIFI = 2,
    UWB = 3,
}

impl fmt::Display for ChipKindEnum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Chip {
    kind: ChipKindEnum,
    id: Option<String>,
    manufacturer: Option<String>,
    product_name: Option<String>,
    #[serde(rename = "fdIn")]
    fd_in: u32,
    #[serde(rename = "fdOut")]
    fd_out: u32,
    loopback: Option<bool>,
}

struct FdTransport {
    file: File,
}

impl Response for FdTransport {
    fn response(&mut self, packet: &cxx::CxxVector<u8>, packet_type: u8) {
        let mut buffer = Vec::<u8>::with_capacity(packet.len() + 1);
        buffer.push(packet_type);
        buffer.extend(packet);
        if let Err(e) = self.file.write_all(&buffer[..]) {
            error!("netsimd: error writing {}", e);
        }
    }
}

/// read from the raw fd and pass to the packet hub.
///
/// # Safety
///
/// `fd_rx` must be a valid and open file descriptor.
unsafe fn fd_reader(
    fd_rx: i32,
    kind: ChipKindEnum,
    facade_id: u32,
    device_id: u32,
    chip_id: u32,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name(format!("fd_reader_{}", fd_rx))
        .spawn(move || {
            // SAFETY: The caller promises that `fd_rx` is valid and open.
            let mut rx = unsafe { File::from_raw_fd(fd_rx) };

            info!("Handling fd={} for kind={:?} facade_id={:?}", fd_rx, kind, facade_id);

            loop {
                match kind {
                    ChipKindEnum::UWB => match uci::read_uci_packet(&mut rx) {
                        Err(e) => {
                            error!("End reader connection with fd={}. Failed to reading uci control packet: {:?}", fd_rx, e);
                            break;
                        }
                        Ok(uci::Packet { payload }) => {
                            handle_request_cxx(kind as u32, facade_id, &payload, 0);
                        }
                    },
                    ChipKindEnum::BLUETOOTH => match h4::read_h4_packet(&mut rx) {
                        Ok(h4::Packet { h4_type, payload }) => {
                            handle_request_cxx(kind as u32, facade_id, &payload, h4_type);
                        }
                        Err(PacketError::IoError(e))
                            if e.kind() == ErrorKind::UnexpectedEof =>
                        {
                            info!("End reader connection with fd={}.", fd_rx);
                            break;
                        }
                        Err(e) => {
                            error!("End reader connection with fd={}. Failed to reading hci control packet: {:?}", fd_rx, e);
                            break;
                        }
                    },
                    _ => {
                        error!("unknown control packet kind: {:?}", kind);
                        break;
                    }
                };
            }

            if let Err(err) = remove_chip(device_id as i32, chip_id as i32) {
                warn!("{err}");
            }
            // File is automatically closed when it goes out of scope.
            unregister_transport(kind as u32, facade_id);
        })
        .unwrap()
}

/// start_fd_transport
///
/// Create threads to read and write to file descriptors
///
/// # Safety
///
/// The file descriptors in the JSON must be valid and open.
pub unsafe fn run_fd_transport(startup_json: &String) {
    info!("Running fd transport with {startup_json}");
    let startup_info = match serde_json::from_str::<StartupInfo>(startup_json.as_str()) {
        Err(e) => {
            error!("Error parsing startup info: {:?}", e);
            return;
        }
        Ok(startup_info) => startup_info,
    };
    // See https://tokio.rs/tokio/topics/bridging
    // This code is synchronous hosting asynchronous until main is converted to rust.
    thread::Builder::new()
        .name("fd_transport".to_string())
        .spawn(move || {
            let chip_count = startup_info.devices.iter().map(|d| d.chips.len()).sum();
            let mut handles = Vec::with_capacity(chip_count);
            for device in startup_info.devices {
                for chip in device.chips {
                    let chip_kind = match chip.kind {
                        ChipKindEnum::BLUETOOTH => ChipKind::BLUETOOTH,
                        ChipKindEnum::WIFI => ChipKind::WIFI,
                        ChipKindEnum::UWB => ChipKind::UWB,
                        _ => ChipKind::UNSPECIFIED,
                    };
                    let result = match add_chip(
                        &chip.fd_in.to_string(),
                        &device.name.clone(),
                        chip_kind,
                        &chip.id.unwrap_or_default(),
                        &chip.manufacturer.unwrap_or_default(),
                        &chip.product_name.unwrap_or_default(),
                    ) {
                        Ok(chip_result) => chip_result,
                        Err(err) => {
                            warn!("{err}");
                            return;
                        }
                    };

                    // Cf writes to fd_out and reads from fd_in
                    // SAFETY: Our caller promises that the file descriptors in the JSON are valid
                    // and open.
                    let file_in = unsafe { File::from_raw_fd(chip.fd_in as i32) };

                    register_transport(
                        chip.kind as u32,
                        result.facade_id,
                        Box::new(FdTransport { file: file_in }),
                    );
                    // TODO: switch to runtime.spawn once FIFOs are available in Tokio
                    // SAFETY: Our caller promises that the file descriptors in the JSON are valid
                    // and open.
                    handles.push(unsafe {
                        fd_reader(
                            chip.fd_out as i32,
                            chip.kind,
                            result.facade_id,
                            result.device_id as u32,
                            result.chip_id as u32,
                        )
                    });
                }
            }
            // Wait for all of them to complete.
            for handle in handles {
                // The `spawn` method returns a `JoinHandle`. A `JoinHandle` is
                // a future, so we can wait for it using `block_on`.
                // runtime.block_on(handle).unwrap();
                // TODO: use runtime.block_on once FIFOs are available in Tokio
                handle.join().unwrap();
            }
            info!("done with all fd handlers");
        })
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::StartupInfo;
    use log::info;

    #[test]
    fn test_serde() {
        let s = r#"
    {"devices": [
       {"name": "emulator-5554",
        "chips": [{"kind": "WIFI", "fdIn": 1, "fdOut": 2},
                  {"kind": "BLUETOOTH", "fdIn": 20, "fdOut":21}]
       },
       {"name": "emulator-5555",
        "chips": [{"kind": "BLUETOOTH", "fdIn": 3, "fdOut": 4},
                {"kind": "UWB", "fdIn": 5, "fdOut": 6, "model": "DW300"}]
       }
     ]
    }"#;
        let startup_info = serde_json::from_str::<StartupInfo>(s).unwrap();
        for device in startup_info.devices {
            info!("device {:?}", device);
        }
    }
}
