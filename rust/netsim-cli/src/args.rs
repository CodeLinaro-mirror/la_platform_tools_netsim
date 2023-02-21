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

use clap::{Args, Parser, Subcommand, ValueEnum};
use frontend_proto::frontend;
use frontend_proto::model;
use frontend_proto::model::State;
use frontend_proto::model::{Chip_Bluetooth, Chip_Radio};
use protobuf::Message;

#[derive(Debug, Parser)]
pub struct NetsimArgs {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print Netsim version information
    Version,
    /// Control the radio state of a device
    Radio(Radio),
    /// Set the device location
    Move(Move),
    /// Display device(s) information
    Devices,
    /// Control the packet capture for one or all devices
    Capture(Capture),
    /// Reset Netsim device scene
    Reset,
    /// Open netsim Web UI
    Gui,
}

impl Command {
    /// Return the generated request protobuf as a byte vector
    /// The parsed command parameters are used to construct the request protobuf which is
    /// returned as a byte vector that can be sent to the server.
    pub fn get_request_bytes(&self) -> Vec<u8> {
        match self {
            Command::Version => Vec::new(),
            Command::Radio(cmd) => {
                let mut result = frontend::UpdateDeviceRequest::new();
                let mutable_device = result.mut_device();
                mutable_device.set_device_serial(cmd.device_serial.to_owned());
                let mutable_chips = mutable_device.mut_chips();
                mutable_chips.push_default();
                let mut bt_chip = Chip_Bluetooth::new();
                let chip_state = match cmd.status {
                    UpDownStatus::Up => State::ON,
                    UpDownStatus::Down => State::OFF,
                };
                if cmd.bt_type == BtType::Ble {
                    bt_chip.set_low_energy(Chip_Radio { state: chip_state, ..Default::default() });
                } else {
                    bt_chip.set_classic(Chip_Radio { state: chip_state, ..Default::default() });
                }
                mutable_chips[0].set_bt(bt_chip);
                result.write_to_bytes().unwrap()
            }
            Command::Move(cmd) => {
                let mut result = frontend::UpdateDeviceRequest::new();
                let mutable_device = result.mut_device();
                mutable_device.set_device_serial(cmd.device_serial.to_owned());
                mutable_device.set_position(model::Position {
                    x: cmd.x,
                    y: cmd.y,
                    z: cmd.z.unwrap_or_default(),
                    ..Default::default()
                });
                result.write_to_bytes().unwrap()
            }
            Command::Devices => Vec::new(),
            Command::Capture(cmd) => {
                let mut result = frontend::SetPacketCaptureRequest::new();
                result.set_device_serial(cmd.device_serial.to_owned());
                result.set_capture(cmd.state == BoolState::True);
                result.write_to_bytes().unwrap()
            }
            Command::Reset => Vec::new(),
            Command::Gui => {
                unimplemented!("get_request_bytes is not implemented for Gui Command.");
            }
        }
    }
}

#[derive(Debug, Args)]
pub struct Radio {
    /// Radio type
    #[clap(value_enum)]
    pub bt_type: BtType,
    /// Radio status
    #[clap(value_enum)]
    pub status: UpDownStatus,
    /// Device serial
    pub device_serial: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BtType {
    Ble,
    Classic,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum UpDownStatus {
    Up,
    Down,
}

#[derive(Debug, Args)]
pub struct Move {
    /// Device serial
    pub device_serial: String,
    /// x position of device
    pub x: f32,
    /// y position of device
    pub y: f32,
    /// Optional z position of device
    pub z: Option<f32>,
}

#[derive(Debug, Args)]
pub struct Capture {
    /// Capture state
    #[clap(value_enum)]
    pub state: BoolState,
    /// Device serial
    pub device_serial: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BoolState {
    // NOTE: Temporarily disable this attribute because clap-3.2.22 is used.
    // #[value(alias("True"), alias("TRUE"))]
    True,
    // #[value(alias("False"), alias("FALSE"))]
    False,
}
