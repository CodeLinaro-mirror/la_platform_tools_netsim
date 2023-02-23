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

use crate::args::{self, Command};
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
            Command::Radio(_) => {
                todo!();
            }
            Command::Move(_) => {
                todo!();
            }
            Command::Devices => {
                Self::print_device_response(
                    GetDevicesResponse::parse_from_bytes(response).unwrap(),
                );
            }
            Command::Capture(_) => {
                todo!();
            }
            Command::Reset => {
                todo!();
            }
            Command::Gui => {
                unimplemented!("No Grpc Response for Gui Command.");
            }
        }
    }

    /// Helper function to format and print GetDevicesResponse
    fn print_device_response(response: GetDevicesResponse) {
        println!("List of devices attached");
        for device in response.devices {
            print!("{}\t", device.device_serial);
            for chip in &device.chips {
                match &chip.chip {
                    Some(Chip_oneof_chip::bt(bt)) => {
                        print!(
                            "ble: {}\t",
                            Self::bt_state_to_string(bt.get_low_energy().get_state())
                        );
                        print!(
                            "classic: {}\t",
                            Self::bt_state_to_string(bt.get_classic().get_state())
                        );
                    }
                    _ => print!("Unknown: down/t"),
                }
            }
            let pos = device.get_position();
            println!("position ({:.2}, {:.2}, {:.2})", pos.get_x(), pos.get_y(), pos.get_z());
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
