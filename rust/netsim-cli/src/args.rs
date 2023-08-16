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
use frontend_client_cxx::ffi::{FrontendClient, GrpcMethod};
use frontend_proto::common::ChipKind;
use frontend_proto::frontend;
use frontend_proto::frontend::patch_capture_request::PatchCapture as PatchCaptureProto;
use frontend_proto::model::chip::bluetooth_beacon::advertise_settings::Interval;
use frontend_proto::model::chip::bluetooth_beacon::{AdvertiseData, AdvertiseSettings};
use frontend_proto::model::chip::{
    Bluetooth as Chip_Bluetooth, BluetoothBeacon as Chip_Ble_Beacon, Chip as Chip_Type,
    Radio as Chip_Radio,
};
use frontend_proto::model::{
    self, chip_create, Chip, ChipCreate as ChipCreateProto, Device,
    DeviceCreate as DeviceCreateProto, Position, State,
};
use log::error;
use netsim_common::util::time_display::TimeDisplay;
use protobuf::{Message, MessageField};
use std::fmt;

pub type BinaryProtobuf = Vec<u8>;

#[derive(Debug, Parser)]
pub struct NetsimArgs {
    #[command(subcommand)]
    pub command: Command,
    /// Set verbose mode
    #[arg(short, long)]
    pub verbose: bool,
    /// Set custom grpc port
    #[arg(short, long)]
    pub port: Option<i32>,
    /// Set netsimd instance number to connect
    #[arg(short, long)]
    pub instance: Option<u16>,
}

#[derive(Debug, Subcommand)]
#[command(infer_subcommands = true)]
pub enum Command {
    /// Print Netsim version information
    Version,
    /// Control the radio state of a device
    Radio(Radio),
    /// Set the device location
    Move(Move),
    /// Display device(s) information
    Devices(Devices),
    /// Reset Netsim device scene
    Reset,
    /// Open netsim Web UI
    Gui,
    /// Control the packet capture functionalities with commands: list, patch, get
    #[command(subcommand, visible_alias("pcap"))]
    Capture(Capture),
    /// Opens netsim artifacts directory (log, pcaps)
    Artifact,
    /// A chip that sends advertisements at a set interval
    #[command(subcommand)]
    Beacon(Beacon),
}

impl Command {
    /// Return the generated request protobuf as a byte vector
    /// The parsed command parameters are used to construct the request protobuf which is
    /// returned as a byte vector that can be sent to the server.
    pub fn get_request_bytes(&self) -> BinaryProtobuf {
        match self {
            Command::Version => Vec::new(),
            Command::Radio(cmd) => {
                let mut chip = Chip { ..Default::default() };
                let chip_state = match cmd.status {
                    UpDownStatus::Up => State::ON,
                    UpDownStatus::Down => State::OFF,
                };
                if cmd.radio_type == RadioType::Wifi {
                    let mut wifi_chip = Chip_Radio::new();
                    wifi_chip.state = chip_state.into();
                    chip.set_wifi(wifi_chip);
                    chip.kind = ChipKind::WIFI.into();
                } else if cmd.radio_type == RadioType::Uwb {
                    let mut uwb_chip = Chip_Radio::new();
                    uwb_chip.state = chip_state.into();
                    chip.set_uwb(uwb_chip);
                    chip.kind = ChipKind::UWB.into();
                } else {
                    let mut bt_chip = Chip_Bluetooth::new();
                    let mut bt_chip_radio = Chip_Radio::new();
                    bt_chip_radio.state = chip_state.into();
                    if cmd.radio_type == RadioType::Ble {
                        bt_chip.low_energy = Some(bt_chip_radio).into();
                    } else {
                        bt_chip.classic = Some(bt_chip_radio).into();
                    }
                    chip.kind = ChipKind::BLUETOOTH.into();
                    chip.set_bt(bt_chip);
                }
                let mut result = frontend::PatchDeviceRequest::new();
                let mut device = Device::new();
                device.name = cmd.name.to_owned();
                device.chips.push(chip);
                result.device = Some(device).into();
                result.write_to_bytes().unwrap()
            }
            Command::Move(cmd) => {
                let mut result = frontend::PatchDeviceRequest::new();
                let mut device = Device::new();
                let position = Position {
                    x: cmd.x,
                    y: cmd.y,
                    z: cmd.z.unwrap_or_default(),
                    ..Default::default()
                };
                device.name = cmd.name.to_owned();
                device.position = Some(position).into();
                result.device = Some(device).into();
                result.write_to_bytes().unwrap()
            }
            Command::Devices(_) => Vec::new(),
            Command::Reset => Vec::new(),
            Command::Gui => {
                unimplemented!("get_request_bytes is not implemented for Gui Command.");
            }
            Command::Capture(cmd) => match cmd {
                Capture::List(_) => Vec::new(),
                Capture::Get(_) => {
                    unimplemented!("get_request_bytes not implemented for Capture Get command. Use get_requests instead.")
                }
                Capture::Patch(_) => {
                    unimplemented!("get_request_bytes not implemented for Capture Patch command. Use get_requests instead.")
                }
            },
            Command::Artifact => {
                unimplemented!("get_request_bytes is not implemented for Artifact Command.")
            }
            Command::Beacon(action) => match action {
                Beacon::Create(kind) => match kind {
                    BeaconCreate::Ble(args) => {
                        let device = MessageField::some(DeviceCreateProto {
                            name: args.device_name.clone().unwrap_or_default(),
                            chips: vec![ChipCreateProto {
                                name: args.chip_name.clone().unwrap_or_default(),
                                kind: ChipKind::BLUETOOTH_BEACON.into(),
                                chip: Some(chip_create::Chip::BleBeacon(
                                    chip_create::BluetoothBeaconCreate {
                                        settings: MessageField::some(AdvertiseSettings {
                                            interval: args
                                                .settings
                                                .interval
                                                .map(Interval::Milliseconds),
                                            ..Default::default()
                                        }),
                                        adv_data: MessageField::some(AdvertiseData {
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                )),
                                ..Default::default()
                            }],
                            ..Default::default()
                        });

                        let result = frontend::CreateDeviceRequest { device, ..Default::default() };
                        result.write_to_bytes().unwrap()
                    }
                },
                Beacon::Patch(kind) => match kind {
                    BeaconPatch::Ble(args) => {
                        let device = MessageField::some(Device {
                            name: args.device_name.clone(),
                            chips: vec![Chip {
                                name: args.chip_name.clone(),
                                kind: ChipKind::BLUETOOTH_BEACON.into(),
                                chip: Some(Chip_Type::BleBeacon(Chip_Ble_Beacon {
                                    bt: MessageField::some(Chip_Bluetooth::new()),
                                    settings: MessageField::some(AdvertiseSettings {
                                        interval: args
                                            .settings
                                            .interval
                                            .map(Interval::Milliseconds),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                })),
                                ..Default::default()
                            }],
                            ..Default::default()
                        });

                        let result = frontend::PatchDeviceRequest { device, ..Default::default() };
                        result.write_to_bytes().unwrap()
                    }
                },
                Beacon::Remove(_) => {
                    todo!("get_request_bytes is not yet implemented for beacon remove command.")
                }
            },
        }
    }

    /// Create and return the request protobuf(s) for the command.
    /// In the case of a command with pattern argument(s) there may be multiple gRPC requests.
    /// The parsed command parameters are used to construct the request protobuf.
    /// The client is used to send gRPC call(s) to retrieve information needed for request protobufs.
    pub fn get_requests(&mut self, client: &cxx::UniquePtr<FrontendClient>) -> Vec<BinaryProtobuf> {
        match self {
            Command::Capture(Capture::Patch(cmd)) => {
                let mut reqs = Vec::new();
                let filtered_captures = Self::get_filtered_captures(client, &cmd.patterns);
                // Create a request for each capture
                for capture in &filtered_captures {
                    let mut result = frontend::PatchCaptureRequest::new();
                    result.id = capture.id;
                    let capture_state = match cmd.state {
                        OnOffState::On => State::ON,
                        OnOffState::Off => State::OFF,
                    };
                    let mut patch_capture = PatchCaptureProto::new();
                    patch_capture.state = capture_state.into();
                    result.patch = Some(patch_capture).into();
                    reqs.push(result.write_to_bytes().unwrap())
                }
                reqs
            }
            Command::Capture(Capture::Get(cmd)) => {
                let mut reqs = Vec::new();
                let filtered_captures = Self::get_filtered_captures(client, &cmd.patterns);
                // Create a request for each capture
                for capture in &filtered_captures {
                    let mut result = frontend::GetCaptureRequest::new();
                    result.id = capture.id;
                    reqs.push(result.write_to_bytes().unwrap());
                    let time_display = TimeDisplay::new(
                        capture.timestamp.get_or_default().seconds,
                        capture.timestamp.get_or_default().nanos as u32,
                    );
                    let file_extension = "pcap";
                    cmd.filenames.push(format!(
                        "{:?}-{}-{}-{}.{}",
                        capture.id,
                        capture.device_name.to_owned().replace(' ', "_"),
                        Self::chip_kind_to_string(capture.chip_kind.enum_value_or_default()),
                        time_display.utc_display(),
                        file_extension
                    ));
                }
                reqs
            }
            _ => {
                unimplemented!(
                    "get_requests not implemented for this command. Use get_request_bytes instead."
                )
            }
        }
    }

    fn get_filtered_captures(
        client: &cxx::UniquePtr<FrontendClient>,
        patterns: &Vec<String>,
    ) -> Vec<model::Capture> {
        // Get list of captures
        let result = client.send_grpc(&GrpcMethod::ListCapture, &Vec::new());
        if !result.is_ok() {
            error!("ListCapture Grpc call error: {}", result.err());
            return Vec::new();
        }
        let mut response =
            frontend::ListCaptureResponse::parse_from_bytes(result.byte_vec().as_slice()).unwrap();
        if !patterns.is_empty() {
            // Filter out list of captures with matching patterns
            Self::filter_captures(&mut response.captures, patterns)
        }
        response.captures
    }
}

#[derive(Debug, Args)]
pub struct Radio {
    /// Radio type
    #[arg(value_enum, ignore_case = true)]
    pub radio_type: RadioType,
    /// Radio status
    #[arg(value_enum, ignore_case = true)]
    pub status: UpDownStatus,
    /// Device name
    pub name: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum RadioType {
    Ble,
    Classic,
    Wifi,
    Uwb,
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
    #[arg(short, long)]
    pub continuous: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OnOffState {
    On,
    Off,
}

#[derive(Debug, Subcommand)]
pub enum Beacon {
    /// Create a beacon chip
    #[command(subcommand)]
    Create(BeaconCreate),
    /// Modify a beacon chip
    #[command(subcommand)]
    Patch(BeaconPatch),
    /// Remove a beacon chip
    Remove(BeaconRemove),
}

#[derive(Debug, Subcommand)]
pub enum BeaconCreate {
    /// Create a Bluetooth low-energy beacon chip
    Ble(BeaconCreateBle),
}

#[derive(Debug, Args)]
pub struct BeaconCreateBle {
    /// Name of the device to create
    pub device_name: Option<String>,
    /// Name of the beacon chip to create within the new device. May only be specified if device_name is specified
    pub chip_name: Option<String>,
    #[command(flatten)]
    pub settings: BeaconBleSettings,
}

#[derive(Debug, Subcommand)]
pub enum BeaconPatch {
    /// Modify a Bluetooth low-energy beacon chip
    Ble(BeaconPatchBle),
}

#[derive(Debug, Args)]
pub struct BeaconPatchBle {
    /// Name of the device that contains the chip
    pub device_name: String,
    /// Name of the beacon chip to modify
    pub chip_name: String,
    #[command(flatten)]
    #[group(required = true, multiple = true)]
    pub settings: BeaconBleSettings,
}

#[derive(Debug, Args)]
pub struct BeaconRemove {
    /// Name of the device to remove
    pub device_name: String,
    /// Name of the beacon chip to remove
    pub chip_name: Option<String>,
}

#[derive(Debug, Args)]
pub struct BeaconBleSettings {
    /// Duration between advertisements in ms
    #[arg(long)]
    pub interval: Option<u64>,
}

#[derive(Debug, Subcommand)]
pub enum Capture {
    /// List currently available Captures (packet captures)
    List(ListCapture),
    /// Patch a Capture source to turn packet capture on/off
    Patch(PatchCapture),
    /// Download the packet capture content
    Get(GetCapture),
}

#[derive(Debug, Args)]
pub struct ListCapture {
    /// Optional strings of pattern for captures to list. Possible filter fields include Capture ID, Device Name, and Chip Kind
    pub patterns: Vec<String>,
    /// Continuously print Capture information every second
    #[arg(short, long)]
    pub continuous: bool,
}

#[derive(Debug, Args)]
pub struct PatchCapture {
    /// Packet capture state
    #[arg(value_enum, ignore_case = true)]
    pub state: OnOffState,
    /// Optional strings of pattern for captures to patch. Possible filter fields include Capture ID, Device Name, and Chip Kind
    pub patterns: Vec<String>,
}

#[derive(Debug, Args)]
pub struct GetCapture {
    /// Optional strings of pattern for captures to get. Possible filter fields include Capture ID, Device Name, and Chip Kind
    pub patterns: Vec<String>,
    /// Directory to store downloaded capture file(s)
    #[arg(short = 'o', long)]
    pub location: Option<String>,
    #[arg(skip)]
    pub filenames: Vec<String>,
    #[arg(skip)]
    pub current_file: String,
}
