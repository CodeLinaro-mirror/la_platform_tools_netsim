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

use crate::args::{self, Command, OnOffState, Pcap};
use frontend_proto::{
    frontend::ListPcapResponse,
    frontend::{GetDevicesResponse, VersionResponse},
    model::Chip_oneof_chip,
    model::State,
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
                        cmd.radio_type,
                        cmd.status,
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
                        Self::on_off_state_to_string(cmd.state),
                        cmd.name.to_owned()
                    );
                }
            }
            Command::Reset => {
                if verbose {
                    println!("All devices have been reset.");
                }
            }
            Command::Pcap(Pcap::List) => Self::print_list_pcap_response(
                ListPcapResponse::parse_from_bytes(response).unwrap(),
                verbose,
            ),
            Command::Pcap(Pcap::Patch(cmd)) => {
                if verbose {
                    println!(
                        "Patched Pcap id: {} to {}",
                        cmd.id,
                        Self::on_off_state_to_string(cmd.state)
                    )
                }
            }
            Command::Pcap(Pcap::Get(_)) => {
                // TODO: Add output with downloaded file information
                todo!("GetPcap response output not yet implemented.")
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
        let radio_width = 9;
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
                                    "{:chip_indent$}{:radio_width$}{:state_width$}| rx_count: {:cnt_width$?} | tx_count: {:cnt_width$?} | capture: {}",
                                    "",
                                    "ble:",
                                    Self::chip_state_to_string(ble_chip.get_state()),
                                    ble_chip.get_rx_count(),
                                    ble_chip.get_tx_count(),
                                    Self::capture_state_to_string(chip.capture)
                                );
                            }
                            if bt.has_classic() {
                                let classic_chip = bt.get_classic();
                                println!(
                                    "{:chip_indent$}{:radio_width$}{:state_width$}| rx_count: {:cnt_width$?} | tx_count: {:cnt_width$?} | capture: {}",
                                    "",
                                    "classic:",
                                    Self::chip_state_to_string(classic_chip.get_state()),
                                    classic_chip.get_rx_count(),
                                    classic_chip.get_tx_count(),
                                    Self::capture_state_to_string(chip.capture)
                                );
                            }
                        }
                        Some(Chip_oneof_chip::wifi(wifi_chip)) => {
                            println!(
                                "{:chip_indent$}{:radio_width$}{:state_width$}| rx_count: {:cnt_width$?} | tx_count: {:cnt_width$?} | capture: {}",
                                "",
                                "wifi:",
                                Self::chip_state_to_string(wifi_chip.get_state()),
                                wifi_chip.get_rx_count(),
                                wifi_chip.get_tx_count(),
                                Self::capture_state_to_string(chip.capture)
                            );
                        }
                        _ => println!("{:chip_indent$}Unknown chip: down  ", ""),
                    }
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
                    match &chip.chip {
                        Some(Chip_oneof_chip::bt(bt)) => {
                            if bt.has_low_energy() {
                                let ble_chip = bt.get_low_energy();
                                if ble_chip.get_state() == State::OFF {
                                    print!(
                                        "{:chip_indent$}{:radio_width$}{:state_width$}",
                                        "",
                                        "ble:",
                                        Self::chip_state_to_string(ble_chip.get_state()),
                                    );
                                }
                            }
                            if bt.has_classic() {
                                let classic_chip = bt.get_classic();
                                if classic_chip.get_state() == State::OFF {
                                    print!(
                                        "{:chip_indent$}{:radio_width$}{:state_width$}",
                                        "",
                                        "classic:",
                                        Self::chip_state_to_string(classic_chip.get_state())
                                    );
                                }
                            }
                        }
                        Some(Chip_oneof_chip::wifi(wifi_chip)) => {
                            if wifi_chip.get_state() == State::OFF {
                                print!(
                                    "{:chip_indent$}{:radio_width$}{:state_width$}",
                                    "",
                                    "wifi:",
                                    Self::chip_state_to_string(wifi_chip.get_state())
                                );
                            }
                        }
                        _ => {}
                    }
                    if chip.capture == State::ON {
                        print!("{:chip_indent$}capture: on", "");
                    }
                }
                println!();
            }
        }
    }

    /// Helper function to convert frontend_proto::model::State to string for output
    fn chip_state_to_string(state: State) -> String {
        match state {
            State::ON => "up".to_string(),
            State::OFF => "down".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn capture_state_to_string(state: State) -> String {
        match state {
            State::ON => "on".to_string(),
            State::OFF => "off".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn on_off_state_to_string(state: OnOffState) -> String {
        match state {
            OnOffState::On => "on".to_string(),
            OnOffState::Off => "off".to_string(),
        }
    }

    /// Helper function to format and print VersionResponse
    fn print_version_response(response: VersionResponse) {
        println!("Netsim version: {}", response.version);
    }

    /// Helper function to format and print ListPcapResponse
    fn print_list_pcap_response(response: ListPcapResponse, verbose: bool) {
        let id_width = 4;
        let name_width = 16;
        let state_width = 5;
        if response.pcaps.is_empty() {
            println!("No available Pcaps found.");
        } else {
            println!("List of Pcaps:");
        }
        for pcap in &response.pcaps {
            // TODO: Enhance output with additional information once implemented
            if verbose || !pcap.state {
                println!(
                    "Pcap ID: {:id_width$}, Device: {:name_width$}, State: {:state_width$}",
                    pcap.id.to_string(),
                    pcap.device_name,
                    if pcap.state { "on" } else { "off" }
                );
            }
        }
    }
}
