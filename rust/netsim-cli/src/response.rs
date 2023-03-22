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

use std::cmp::max;

use crate::args::{self, Command, OnOffState, Pcap};
use frontend_proto::{
    common::ChipKind,
    frontend::{GetDevicesResponse, ListPcapResponse, VersionResponse},
    model::{self, Chip_oneof_chip, State},
};
use protobuf::{Message, RepeatedField};

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
            Command::Pcap(Pcap::List(cmd)) => Self::print_list_pcap_response(
                ListPcapResponse::parse_from_bytes(response).unwrap(),
                verbose,
                cmd.patterns.to_owned(),
            ),
            Command::Pcap(Pcap::Patch(cmd)) => {
                if verbose {
                    println!("Patched Pcap state to {}", Self::on_off_state_to_string(cmd.state),);
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
    fn print_list_pcap_response(
        mut response: ListPcapResponse,
        verbose: bool,
        patterns: Vec<String>,
    ) {
        if response.pcaps.is_empty() {
            if verbose {
                println!("No available Pcap found.");
            }
            return;
        }
        if patterns.is_empty() {
            println!("List of Pcaps:");
        } else {
            println!("List of Pcaps matching pattern(s) `{:?}`:", patterns);
            // Filter out list of pcaps with matching patterns
            Self::filter_pcaps(&mut response.pcaps, &patterns)
        }
        // Create the header row and determine column widths
        let id_hdr = "ID";
        let name_hdr = "Device Name";
        let chipkind_hdr = "Chip Kind";
        let state_hdr = "State";
        let size_hdr = "Size";
        let id_width = 4; // ID width of 4 since Pcap id starts at 4000
        let state_width = 7; // State width of 7 for 'unknown'
        let chipkind_width = 11; // ChipKind width 11 for 'UNSPECIFIED'
        let name_width = max(
            (response.pcaps.iter().max_by_key(|x| x.device_name.len()))
                .unwrap_or_default()
                .device_name
                .len(),
            name_hdr.len(),
        );
        let size_width = max(
            (response.pcaps.iter().max_by_key(|x| x.size))
                .unwrap_or_default()
                .size
                .to_string()
                .len(),
            size_hdr.len(),
        );
        // Print header for pcap list
        println!(
            "{:id_width$} | {:name_width$} | {:chipkind_width$} | {:state_width$} | {:size_width$} |",
            id_hdr,
            name_hdr,
            chipkind_hdr,
            state_hdr,
            size_hdr,
        );
        // Print information of each Pcap
        for pcap in &response.pcaps {
            println!(
                "{:id_width$} | {:name_width$} | {:chipkind_width$} | {:state_width$} | {:size_width$} |",
                pcap.id.to_string(),
                pcap.device_name,
                Self::chip_kind_to_string(pcap.chip_kind),
                Self::capture_state_to_string(pcap.state),
                pcap.size,
            );
        }
    }

    fn chip_kind_to_string(chip_kind: ChipKind) -> String {
        match chip_kind {
            ChipKind::UNSPECIFIED => "UNSPECIFIED".to_string(),
            ChipKind::BLUETOOTH => "BLUETOOTH".to_string(),
            ChipKind::WIFI => "WIFI".to_string(),
            ChipKind::UWB => "UWB".to_string(),
        }
    }

    pub fn filter_pcaps(pcaps: &mut RepeatedField<model::Pcap>, keys: &[String]) {
        // Filter out list of pcaps with matching pattern
        pcaps.retain(|pcap| {
            keys.iter().map(|key| key.to_uppercase()).all(|key| {
                pcap.id.to_string().contains(&key)
                    || pcap.device_name.to_uppercase().contains(&key)
                    || Self::chip_kind_to_string(pcap.chip_kind).contains(&key)
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_filter_pcaps_helper(patterns: Vec<String>, expected_pcaps: RepeatedField<model::Pcap>) {
        let mut pcaps = all_test_pcaps();
        Command::filter_pcaps(&mut pcaps, &patterns);
        assert_eq!(pcaps, expected_pcaps);
    }

    fn pcap_1() -> model::Pcap {
        model::Pcap {
            id: 4001,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "device 1".to_string(),
            ..Default::default()
        }
    }
    fn pcap_1_wifi() -> model::Pcap {
        model::Pcap {
            id: 4002,
            chip_kind: ChipKind::WIFI,
            device_name: "device 1".to_string(),
            ..Default::default()
        }
    }
    fn pcap_2() -> model::Pcap {
        model::Pcap {
            id: 4003,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "device 2".to_string(),
            ..Default::default()
        }
    }
    fn pcap_3() -> model::Pcap {
        model::Pcap {
            id: 4004,
            chip_kind: ChipKind::WIFI,
            device_name: "device 3".to_string(),
            ..Default::default()
        }
    }
    fn all_test_pcaps() -> RepeatedField<model::Pcap> {
        RepeatedField::from_vec(vec![pcap_1(), pcap_1_wifi(), pcap_2(), pcap_3()])
    }

    #[test]
    fn test_no_match() {
        test_filter_pcaps_helper(
            vec!["test".to_string()],
            RepeatedField::<model::Pcap>::from_vec(vec![]),
        );
    }

    #[test]
    fn test_all_match() {
        test_filter_pcaps_helper(vec!["device".to_string()], all_test_pcaps());
    }

    #[test]
    fn test_match_pcap_id() {
        test_filter_pcaps_helper(vec!["4001".to_string()], RepeatedField::from_vec(vec![pcap_1()]));
        test_filter_pcaps_helper(vec!["03".to_string()], RepeatedField::from_vec(vec![pcap_2()]));
        test_filter_pcaps_helper(vec!["40".to_string()], all_test_pcaps());
    }

    #[test]
    fn test_match_device_name() {
        test_filter_pcaps_helper(
            vec!["device 1".to_string()],
            RepeatedField::from_vec(vec![pcap_1(), pcap_1_wifi()]),
        );
        test_filter_pcaps_helper(vec![" 2".to_string()], RepeatedField::from_vec(vec![pcap_2()]));
    }

    #[test]
    fn test_match_device_name_case_insensitive() {
        test_filter_pcaps_helper(
            vec!["DEVICE 1".to_string()],
            RepeatedField::from_vec(vec![pcap_1(), pcap_1_wifi()]),
        );
    }

    #[test]
    fn test_match_wifi() {
        test_filter_pcaps_helper(
            vec!["wifi".to_string()],
            RepeatedField::from_vec(vec![pcap_1_wifi(), pcap_3()]),
        );
        test_filter_pcaps_helper(
            vec!["WIFI".to_string()],
            RepeatedField::from_vec(vec![pcap_1_wifi(), pcap_3()]),
        );
    }

    #[test]
    fn test_match_bt() {
        test_filter_pcaps_helper(
            vec!["BLUETOOTH".to_string()],
            RepeatedField::from_vec(vec![pcap_1(), pcap_2()]),
        );
        test_filter_pcaps_helper(
            vec!["blue".to_string()],
            RepeatedField::from_vec(vec![pcap_1(), pcap_2()]),
        );
    }

    #[test]
    fn test_match_name_and_chip() {
        test_filter_pcaps_helper(
            vec!["device 1".to_string(), "wifi".to_string()],
            RepeatedField::from_vec(vec![pcap_1_wifi()]),
        );
    }
}
