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
use frontend_client_cxx::ffi::GrpcMethod;

impl args::Command {
    /// Return the respective GrpcMethod for the command
    pub fn grpc_method(&self) -> GrpcMethod {
        match self {
            Command::Version => GrpcMethod::GetVersion,
            Command::Radio(_) => GrpcMethod::PatchDevice,
            Command::Move(_) => GrpcMethod::PatchDevice,
            Command::Devices(_) => GrpcMethod::GetDevices,
            Command::Capture(_) => GrpcMethod::PatchDevice,
            Command::Reset => GrpcMethod::Reset,
            Command::Pcap(cmd) => match cmd {
                args::Pcap::List(_) => GrpcMethod::ListPcap,
                args::Pcap::Get(_) => GrpcMethod::GetPcap,
                args::Pcap::Patch(_) => GrpcMethod::PatchPcap,
            },
            Command::Gui => {
                panic!("No GrpcMethod for Ui Command.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use args::{BinaryProtobuf, NetsimArgs, OnOffState};
    use clap::Parser;
    use frontend_proto::{
        common::ChipKind,
        frontend,
        model::{self, Chip_Bluetooth, Chip_Radio, Position, State},
    };
    use protobuf::Message;

    fn test_command(
        command: &str,
        expected_grpc_method: GrpcMethod,
        expected_request_byte_str: BinaryProtobuf,
    ) {
        let command = NetsimArgs::parse_from(command.split_whitespace()).command;
        assert_eq!(expected_grpc_method, command.grpc_method());
        let request = command.get_request_bytes();
        assert_eq!(request, expected_request_byte_str);
    }

    #[test]
    fn test_version_request() {
        test_command("netsim-cli version", GrpcMethod::GetVersion, Vec::new())
    }

    fn get_expected_radio(name: &str, radio_type: &str, state: &str) -> BinaryProtobuf {
        let mut chip = model::Chip { ..Default::default() };
        let chip_state = match state {
            "up" => State::ON,
            _ => State::OFF,
        };
        if radio_type == "wifi" {
            let mut wifi_chip = Chip_Radio::new();
            wifi_chip.set_state(chip_state);
            chip.set_wifi(wifi_chip);
            chip.set_kind(ChipKind::WIFI);
        } else if radio_type == "uwb" {
            let mut uwb_chip = Chip_Radio::new();
            uwb_chip.set_state(chip_state);
            chip.set_uwb(uwb_chip);
            chip.set_kind(ChipKind::UWB);
        } else {
            let mut bt_chip = Chip_Bluetooth::new();
            if radio_type == "ble" {
                bt_chip.set_low_energy(Chip_Radio { state: chip_state, ..Default::default() });
            } else {
                bt_chip.set_classic(Chip_Radio { state: chip_state, ..Default::default() });
            }
            chip.set_kind(ChipKind::BLUETOOTH);
            chip.set_bt(bt_chip);
        }
        let mut result = frontend::PatchDeviceRequest::new();
        let mutable_device = result.mut_device();
        mutable_device.set_name(name.to_owned());
        let mutable_chips = mutable_device.mut_chips();
        mutable_chips.push(chip);
        result.write_to_bytes().unwrap()
    }

    #[test]
    fn test_radio_ble() {
        test_command(
            "netsim-cli radio ble down 1000",
            GrpcMethod::PatchDevice,
            get_expected_radio("1000", "ble", "down"),
        );
        test_command(
            "netsim-cli radio ble up 1000",
            GrpcMethod::PatchDevice,
            get_expected_radio("1000", "ble", "up"),
        );
    }

    #[test]
    fn test_radio_ble_aliases() {
        test_command(
            "netsim-cli radio ble Down 1000",
            GrpcMethod::PatchDevice,
            get_expected_radio("1000", "ble", "down"),
        );
        test_command(
            "netsim-cli radio ble Up 1000",
            GrpcMethod::PatchDevice,
            get_expected_radio("1000", "ble", "up"),
        );
        test_command(
            "netsim-cli radio ble DOWN 1000",
            GrpcMethod::PatchDevice,
            get_expected_radio("1000", "ble", "down"),
        );
        test_command(
            "netsim-cli radio ble UP 1000",
            GrpcMethod::PatchDevice,
            get_expected_radio("1000", "ble", "up"),
        );
    }

    #[test]
    fn test_radio_classic() {
        test_command(
            "netsim-cli radio classic down 100",
            GrpcMethod::PatchDevice,
            get_expected_radio("100", "classic", "down"),
        );
        test_command(
            "netsim-cli radio classic up 100",
            GrpcMethod::PatchDevice,
            get_expected_radio("100", "classic", "up"),
        );
    }

    #[test]
    fn test_radio_wifi() {
        test_command(
            "netsim-cli radio wifi down a",
            GrpcMethod::PatchDevice,
            get_expected_radio("a", "wifi", "down"),
        );
        test_command(
            "netsim-cli radio wifi up b",
            GrpcMethod::PatchDevice,
            get_expected_radio("b", "wifi", "up"),
        );
    }

    #[test]
    fn test_radio_uwb() {
        test_command(
            "netsim-cli radio uwb down a",
            GrpcMethod::PatchDevice,
            get_expected_radio("a", "uwb", "down"),
        );
        test_command(
            "netsim-cli radio uwb up b",
            GrpcMethod::PatchDevice,
            get_expected_radio("b", "uwb", "up"),
        );
    }

    fn get_expected_move(name: &str, x: f32, y: f32, z: Option<f32>) -> BinaryProtobuf {
        let mut result = frontend::PatchDeviceRequest::new();
        let mutable_device = result.mut_device();
        mutable_device.set_name(name.to_owned());
        mutable_device.set_position(Position {
            x,
            y,
            z: z.unwrap_or_default(),
            ..Default::default()
        });
        result.write_to_bytes().unwrap()
    }

    #[test]
    fn test_move_int() {
        test_command(
            "netsim-cli move 1 1 2 3",
            GrpcMethod::PatchDevice,
            get_expected_move("1", 1.0, 2.0, Some(3.0)),
        )
    }

    #[test]
    fn test_move_float() {
        test_command(
            "netsim-cli move 1000 1.2 3.4 5.6",
            GrpcMethod::PatchDevice,
            get_expected_move("1000", 1.2, 3.4, Some(5.6)),
        )
    }

    #[test]
    fn test_move_mixed() {
        test_command(
            "netsim-cli move 1000 1.1 2 3.4",
            GrpcMethod::PatchDevice,
            get_expected_move("1000", 1.1, 2.0, Some(3.4)),
        )
    }

    #[test]
    fn test_move_no_z() {
        test_command(
            "netsim-cli move 1000 1.2 3.4",
            GrpcMethod::PatchDevice,
            get_expected_move("1000", 1.2, 3.4, None),
        )
    }

    #[test]
    fn test_devices() {
        test_command("netsim-cli devices", GrpcMethod::GetDevices, Vec::new())
    }

    fn get_expected_capture(name: &str, state: OnOffState) -> BinaryProtobuf {
        let mut bt_chip = model::Chip {
            kind: ChipKind::BLUETOOTH,
            chip: Some(model::Chip_oneof_chip::bt(Chip_Bluetooth { ..Default::default() })),
            ..Default::default()
        };
        let capture_state = match state {
            OnOffState::On => State::ON,
            OnOffState::Off => State::OFF,
        };
        bt_chip.set_capture(capture_state);
        let mut result = frontend::PatchDeviceRequest::new();
        let mutable_device = result.mut_device();
        mutable_device.set_name(name.to_owned());
        let mutable_chips = mutable_device.mut_chips();
        mutable_chips.push(bt_chip);
        result.write_to_bytes().unwrap()
    }

    #[test]
    fn test_capture_lowercase() {
        test_command(
            "netsim-cli capture on test_device",
            GrpcMethod::PatchDevice,
            get_expected_capture("test_device", OnOffState::On),
        );
        test_command(
            "netsim-cli capture off 1000",
            GrpcMethod::PatchDevice,
            get_expected_capture("1000", OnOffState::Off),
        )
    }

    #[test]
    fn test_capture_mixed_case() {
        test_command(
            "netsim-cli capture On 10",
            GrpcMethod::PatchDevice,
            get_expected_capture("10", OnOffState::On),
        );
        test_command(
            "netsim-cli capture Off 1000",
            GrpcMethod::PatchDevice,
            get_expected_capture("1000", OnOffState::Off),
        )
    }

    #[test]
    fn test_capture_uppercase() {
        test_command(
            "netsim-cli capture ON 1000",
            GrpcMethod::PatchDevice,
            get_expected_capture("1000", OnOffState::On),
        );
        test_command(
            "netsim-cli capture OFF 1000",
            GrpcMethod::PatchDevice,
            get_expected_capture("1000", OnOffState::Off),
        )
    }

    #[test]
    fn test_reset() {
        test_command("netsim-cli reset", GrpcMethod::Reset, Vec::new())
    }

    #[test]
    fn test_pcap_list() {
        test_command("netsim-cli pcap list", GrpcMethod::ListPcap, Vec::new())
    }

    //TODO: Add pcap patch and get tests once able to run tests with cxx definitions
}
