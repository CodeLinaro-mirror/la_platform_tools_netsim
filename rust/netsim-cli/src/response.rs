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
    pub fn print_response(&self, response: &[u8], verbose: bool) {
        match self {
            Command::Version => {
                Self::print_version_response(VersionResponse::parse_from_bytes(response).unwrap());
            }
            Command::Radio(cmd) => {
                if verbose {
                    println!(
                        "Radio {} is {} for {}",
                        if cmd.bt_type == BtType::Ble { "ble" } else { "classic" },
                        if cmd.status == UpDownStatus::Up { "up" } else { "down" },
                        cmd.name.to_owned()
                    );
                }
            }
            Command::Move(cmd) => {
                if verbose {
                    println!(
                        "Moved device:{} to x: {:.2}, y: {:.2}, z: {:.2}",
                        cmd.name,
                        cmd.x,
                        cmd.y,
                        cmd.z.unwrap_or_default()
                    )
                }
            }
            Command::Devices(_) => {
                Self::print_device_response(
                    GetDevicesResponse::parse_from_bytes(response).unwrap(),
                    verbose,
                );
            }
            Command::Capture(cmd) => {
                if verbose {
                    println!(
                        "Turned {} packet capture for {}",
                        if cmd.state == OnOffState::On { "on" } else { "off" },
                        cmd.name.to_owned()
                    );
                }
            }
            Command::Reset => {
                if verbose {
                    println!("All devices have been reset.");
                }
            }
            Command::Gui => {
                unimplemented!("No Grpc Response for Gui Command.");
            }
        }
    }

    /// Helper function to format and print GetDevicesResponse
    fn print_device_response(response: GetDevicesResponse, verbose: bool) {
        let pos_prec = 2;
        let name_width = 16;
        let state_width = 5;
        let cnt_width = 9;
        let chip_indent = 2;
        if verbose {
            if response.devices.is_empty() {
                println!("No attached devices found.");
            } else {
                println!("List of attached devices:");
            }
            for device in response.devices {
                let pos = device.get_position();
                println!(
                    "{:name_width$}  position: {:.pos_prec$}, {:.pos_prec$}, {:.pos_prec$}",
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
                                    "{:chip_indent$}ble:     {:state_width$}| rx_count: {:cnt_width$?} | tx_count: {:cnt_width$?}", "",
                                    Self::bt_state_to_string(ble_chip.get_state()),
                                    ble_chip.get_rx_count(),
                                    ble_chip.get_tx_count()
                                );
                            }
                            if bt.has_classic() {
                                let classic_chip = bt.get_classic();
                                println!(
                                    "{:chip_indent$}classic: {:state_width$}| rx_count: {:cnt_width$?} | tx_count: {:cnt_width$?}", "",
                                    Self::bt_state_to_string(classic_chip.get_state()),
                                    classic_chip.get_rx_count(),
                                    classic_chip.get_tx_count()
                                );
                            }
                        }
                        _ => println!("{:chip_indent$}Unknown chip: down  ", ""),
                    }
                    println!(
                        "{:chip_indent$}capture: {}",
                        "",
                        if chip.capture == State::ON { "on" } else { "off" }
                    );
                }
            }
        } else {
            for device in response.devices {
                let pos = device.get_position();
                print!("{:name_width$}  ", device.name,);
                if pos.get_x() != 0.0 || pos.get_y() != 0.0 || pos.get_z() != 0.0 {
                    print!(
                        "position: {:.pos_prec$}, {:.pos_prec$}, {:.pos_prec$}",
                        pos.get_x(),
                        pos.get_y(),
                        pos.get_z()
                    );
                }
                for chip in &device.chips {
                    if let Some(Chip_oneof_chip::bt(bt)) = &chip.chip {
                        if bt.has_low_energy() {
                            let ble_chip = bt.get_low_energy();
                            if ble_chip.get_state() == State::OFF {
                                print!(
                                    "{:chip_indent$}ble: {:state_width$}",
                                    "",
                                    Self::bt_state_to_string(ble_chip.get_state()),
                                );
                            }
                        }
                        if bt.has_classic() {
                            let classic_chip = bt.get_classic();
                            if classic_chip.get_state() == State::OFF {
                                print!(
                                    "{:chip_indent$}classic: {:state_width$}",
                                    "",
                                    Self::bt_state_to_string(classic_chip.get_state())
                                );
                            }
                        }
                    }

                    if chip.capture == State::ON {
                        print!("{:chip_indent$}capture: on", "");
                    }
                    println!();
                }
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
