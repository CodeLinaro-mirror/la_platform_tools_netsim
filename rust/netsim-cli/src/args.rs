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
use frontend_proto::common::ChipKind;
use frontend_proto::frontend;
use frontend_proto::frontend::PatchPcapRequest_PcapPatch as PcapPatch;
use frontend_proto::model;
use frontend_proto::model::State;
use frontend_proto::model::{Chip_Bluetooth, Chip_Radio};
use protobuf::Message;
use std::fmt;

#[derive(Debug, Parser)]
pub struct NetsimArgs {
    #[clap(subcommand)]
    pub command: Command,
    /// Set verbose mode
    #[clap(short, long)]
    pub verbose: bool,
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
    Devices(Devices),
    /// Control the bluetooth packet capture for one or all devices
    Capture(Capture),
    /// Reset Netsim device scene
    Reset,
    /// Open netsim Web UI
    Gui,
    /// (Not fully implemented)
    /// Control the packet capture functionalities with subcommands: list, patch, get
    #[clap(subcommand)]
    Pcap(Pcap),
}

impl Command {
    /// Return the generated request protobuf as a byte vector
    /// The parsed command parameters are used to construct the request protobuf which is
    /// returned as a byte vector that can be sent to the server.
    pub fn get_request_bytes(&self) -> Vec<u8> {
        match self {
            Command::Version => Vec::new(),
            Command::Radio(cmd) => {
                let mut chip = model::Chip { ..Default::default() };
                let chip_state = match cmd.status {
                    UpDownStatus::Up => State::ON,
                    UpDownStatus::Down => State::OFF,
                };
                if cmd.radio_type == RadioType::Wifi {
                    let mut wifi_chip = Chip_Radio::new();
                    wifi_chip.set_state(chip_state);
                    chip.set_wifi(wifi_chip);
                    chip.set_kind(ChipKind::WIFI);
                } else {
                    let mut bt_chip = Chip_Bluetooth::new();
                    if cmd.radio_type == RadioType::Ble {
                        bt_chip
                            .set_low_energy(Chip_Radio { state: chip_state, ..Default::default() });
                    } else {
                        bt_chip.set_classic(Chip_Radio { state: chip_state, ..Default::default() });
                    }
                    chip.set_kind(ChipKind::BLUETOOTH);
                    chip.set_bt(bt_chip);
                }
                let mut result = frontend::PatchDeviceRequest::new();
                let mutable_device = result.mut_device();
                mutable_device.set_name(cmd.name.to_owned());
                let mutable_chips = mutable_device.mut_chips();
                mutable_chips.push(chip);
                result.write_to_bytes().unwrap()
            }
            Command::Move(cmd) => {
                let mut result = frontend::PatchDeviceRequest::new();
                let mutable_device = result.mut_device();
                mutable_device.set_name(cmd.name.to_owned());
                mutable_device.set_position(model::Position {
                    x: cmd.x,
                    y: cmd.y,
                    z: cmd.z.unwrap_or_default(),
                    ..Default::default()
                });
                result.write_to_bytes().unwrap()
            }
            Command::Devices(_) => Vec::new(),
            Command::Capture(cmd) => {
                let mut bt_chip = model::Chip {
                    kind: ChipKind::BLUETOOTH,
                    chip: Some(model::Chip_oneof_chip::bt(Chip_Bluetooth { ..Default::default() })),
                    ..Default::default()
                };
                let capture_state = match cmd.state {
                    OnOffState::On => State::ON,
                    OnOffState::Off => State::OFF,
                };
                bt_chip.set_capture(capture_state);
                let mut result = frontend::PatchDeviceRequest::new();
                let mutable_device = result.mut_device();
                mutable_device.set_name(cmd.name.to_owned());
                let mutable_chips = mutable_device.mut_chips();
                mutable_chips.push(bt_chip);
                result.write_to_bytes().unwrap()
            }
            Command::Reset => Vec::new(),
            Command::Gui => {
                unimplemented!("get_request_bytes is not implemented for Gui Command.");
            }
            Command::Pcap(pcap_cmd) => match pcap_cmd {
                Pcap::List => Vec::new(),
                Pcap::Get(cmd) => {
                    let mut result = frontend::GetPcapRequest::new();
                    result.set_id(cmd.id);
                    result.write_to_bytes().unwrap()
                }
                Pcap::Patch(cmd) => {
                    let mut result = frontend::PatchPcapRequest::new();
                    result.set_id(cmd.id);
                    let capture_state = match cmd.state {
                        OnOffState::On => State::ON,
                        OnOffState::Off => State::OFF,
                    };
                    let mut pcap_patch = PcapPatch::new();
                    pcap_patch.set_state(capture_state);
                    result.set_patch(pcap_patch);
                    result.write_to_bytes().unwrap()
                }
            },
        }
    }
}

#[derive(Debug, Args)]
pub struct Radio {
    /// Radio type
    #[clap(value_enum)]
    pub radio_type: RadioType,
    /// Radio status
    #[clap(value_enum)]
    pub status: UpDownStatus,
    /// Device name
    pub name: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum RadioType {
    Ble,
    Classic,
    Wifi,
}

impl fmt::Display for RadioType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum UpDownStatus {
    Up,
    Down,
}

impl fmt::Display for UpDownStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Args)]
pub struct Move {
    /// Device name
    pub name: String,
    /// x position of device
    pub x: f32,
    /// y position of device
    pub y: f32,
    /// Optional z position of device
    pub z: Option<f32>,
}

#[derive(Debug, Args)]
pub struct Devices {
    /// Continuously print device(s) information every second
    #[clap(short, long)]
    pub continuous: bool,
}

#[derive(Debug, Args)]
pub struct Capture {
    /// Capture state
    #[clap(value_enum)]
    pub state: OnOffState,
    /// Device name
    pub name: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OnOffState {
    // NOTE: Temporarily disable this attribute because clap-3.2.22 is used.
    // #[value(alias("On"), alias("ON"))]
    On,
    // #[value(alias("Off"), alias("OFF"))]
    Off,
}

#[derive(Debug, Subcommand)]
pub enum Pcap {
    /// List all currently available Pcaps (packet captures)
    List,
    /// Patch a Pcap source to turn packet capture on/off
    Patch(PatchPcap),
    /// Download the packet capture content
    Get(GetPcap),
}

#[derive(Debug, Args)]
pub struct PatchPcap {
    /// Pcap id
    pub id: i32,
    /// Packet capture state
    #[clap(value_enum)]
    pub state: OnOffState,
}

#[derive(Debug, Args)]
pub struct GetPcap {
    /// Pcap id
    pub id: i32,
}
