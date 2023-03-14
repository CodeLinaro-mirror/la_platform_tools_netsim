// Copyright 2022 Google LLC
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

use crate::args::{self, BtType, Command, OnOffState, UpDownStatus};
use frontend_proto::model::State;
use frontend_proto::{
    frontend::{GetDevicesResponse, VersionResponse},
    model::Chip_oneof_chip,
};
use protobuf::Message;

impl args::Command {
    /// Format and print the response received from the frontend server for the command
    pub fn print_response(&self, response: &[u8]) {
        match self {
            Command::Version => {
                Self::print_version_response(VersionResponse::parse_from_bytes(response).unwrap());
            }
            Command::Radio(cmd) => {
                println!(
                    "Radio {} is {} for {}",
                    if cmd.bt_type == BtType::Ble { "ble" } else { "classic" },
                    if cmd.status == UpDownStatus::Up { "up" } else { "down" },
                    cmd.name.to_owned()
                );
            }
            Command::Move(cmd) => {
                println!(
                    "Moved device:{} to x: {:.2}, y: {:.2}, z: {:.2}",
                    cmd.name,
                    cmd.x,
                    cmd.y,
                    cmd.z.unwrap_or_default()
                )
            }
            Command::Devices(_) => {
                Self::print_device_response(
                    GetDevicesResponse::parse_from_bytes(response).unwrap(),
                );
            }
            Command::Capture(cmd) => {
                println!(
                    "Turned {} packet capture for {}",
                    if cmd.state == OnOffState::On { "on" } else { "off" },
                    cmd.name.to_owned()
                );
            }
            Command::Reset => {
                println!("All devices have been reset.");
            }
            Command::Gui => {
                unimplemented!("No Grpc Response for Gui Command.");
            }
        }
    }

    /// Helper function to format and print GetDevicesResponse
    fn print_device_response(response: GetDevicesResponse) {
        if response.devices.is_empty() {
            println!("No attached devices found.");
        } else {
            println!("List of attached devices:");
        }
        for device in response.devices {
            let pos = device.get_position();
            println!(
                "{:15}\tposition: {:.2}, {:.2}, {:.2}",
                device.name,
                pos.get_x(),
                pos.get_y(),
                pos.get_z()
            );
            for chip in &device.chips {
                match &chip.chip {
                    Some(Chip_oneof_chip::bt(bt)) => {
                        if bt.has_low_energy() {
                            let ble_chip = bt.get_low_energy();
                            println!(
                                "\tble:     {:5}| rx_count: {:9?} | tx_count: {:9?}",
                                Self::bt_state_to_string(ble_chip.get_state()),
                                ble_chip.get_rx_count(),
                                ble_chip.get_tx_count()
                            );
                        }
                        if bt.has_classic() {
                            let classic_chip = bt.get_classic();
                            println!(
                                "\tclassic: {:5}| rx_count: {:9?} | tx_count: {:9?}",
                                Self::bt_state_to_string(classic_chip.get_state()),
                                classic_chip.get_rx_count(),
                                classic_chip.get_tx_count()
                            );
                        }
                    }
                    _ => print!("\tUnknown chip: down\t"),
                }
                println!(
                    "\tpacket-capture: {}\t",
                    if chip.capture == State::ON { "on" } else { "off" }
                );
            }
        }
    }

    /// Helper function to convert frontend_proto::model::State to string for output
    fn bt_state_to_string(state: State) -> String {
        match state {
            State::ON => "up".to_string(),
            State::OFF => "down".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Helper function to format and print VersionResponse
    fn print_version_response(response: VersionResponse) {
        println!("Netsim version: {}", response.version)
    }
}
