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
    Ui,
}

impl Command {
    pub fn request_json(self) -> String {
        match self {
            Command::Version => String::from("{}"),
            Command::Radio(_cmd) => {
                let result = frontend::UpdateDeviceRequest::new();
                //TODO: Update request content once bt/hci functions are added and working
                serde_json::to_string(&result).unwrap()
            }
            Command::Move(cmd) => {
                let mut result = frontend::UpdateDeviceRequest::new();
                let mutable_device = result.mut_device();
                mutable_device.set_device_serial(cmd.device_serial);
                mutable_device.set_position(model::Position {
                    x: cmd.x,
                    y: cmd.y,
                    z: cmd.z.unwrap_or_default(),
                    ..Default::default()
                });
                serde_json::to_string(&result).unwrap()
            }
            Command::Devices => String::from("{}"),
            Command::Capture(cmd) => {
                let mut result = frontend::SetPacketCaptureRequest::new();
                result.set_device_serial(cmd.device_serial);
                result.set_capture(cmd.state == BoolState::True);
                serde_json::to_string(&result).unwrap()
            }
            Command::Reset => String::from("{}"),
            Command::Ui => {
                panic!("get_json is not implemented for Ui Command.");
            }
        }
    }
}

#[derive(Debug, Args)]
pub struct Radio {
    /// Radio type
    pub bt_type: BtType,
    /// Radio status (up/down)
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
    /// Capture state (true/false)
    pub state: BoolState,
    /// Device serial
    pub device_serial: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BoolState {
    #[value(alias("True"), alias("TRUE"))]
    True,
    #[value(alias("False"), alias("FALSE"))]
    False,
}
