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

use clap::{arg_enum, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct NetsimArgs {
    #[clap(subcommand)]
    pub command: CommandType,
}

#[derive(Debug, Subcommand)]
pub enum CommandType {
    /// Print Netsim version information
    Version,
    /// Control the radio state of a device
    Radio(RadioCommand),
    /// Set the device location
    Move(MoveCommand),
    /// Display device(s) information
    Devices,
    /// Control the packet capture for one or all devices
    Capture(CaptureCommand),
    /// Reset Netsim device scene
    Reset,
    /// Open netsim Web UI
    Ui,
}

#[derive(Debug, Args)]
pub struct RadioCommand {
    /// Radio type
    pub bt_type: BtType,
    /// Radio status (up/down)
    pub status: UpDownStatus,
    /// Device serial
    pub device_serial: String,
}

arg_enum! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum BtType {
        Ble,
        Classic,
    }
}

arg_enum! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum UpDownStatus {
        Up,
        Down,
    }
}

#[derive(Debug, Args)]
pub struct MoveCommand {
    /// Device serial
    pub device_serial: String,
    /// x position of device
    pub x: f32,
    /// y position of device
    pub y: f32,
    /// z position of device
    pub z: f32,
}

#[derive(Debug, Args)]
pub struct CaptureCommand {
    /// Capture state (true/false)
    pub state: BoolState,
    /// Device serial
    pub device_serial: String,
}

arg_enum! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum BoolState {
        True,
        False,
    }
}
