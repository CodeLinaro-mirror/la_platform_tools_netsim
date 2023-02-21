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
use frontend_client_cxx::GrpcMethod;

impl args::Command {
    /// Return the respective GrpcMethod for the command
    pub fn grpc_method(&self) -> GrpcMethod {
        match self {
            Command::Version => GrpcMethod::GetVersion,
            Command::Radio(_) => GrpcMethod::UpdateDevice,
            Command::Move(_) => GrpcMethod::UpdateDevice,
            Command::Devices => GrpcMethod::GetDevices,
            Command::Capture(_) => GrpcMethod::SetPacketCapture,
            Command::Reset => GrpcMethod::Reset,
            Command::Gui => {
                panic!("No GrpcMethod for Ui Command.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use args::NetsimArgs;
    use clap::Parser;
    use frontend_proto::{
        frontend,
        model::{Chip_Bluetooth, Chip_Radio, Position, State},
    };
    use protobuf::Message;

    fn test_command(
        command: &str,
        expected_grpc_method: GrpcMethod,
        expected_request_byte_str: Vec<u8>,
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

    fn get_expected_radio(device_serial: &str, bt_type: &str, state: &str) -> Vec<u8> {
        let mut result = frontend::UpdateDeviceRequest::new();
        let mutable_device = result.mut_device();
        mutable_device.set_device_serial(device_serial.to_owned());
        let mutable_chips = mutable_device.mut_chips();
        mutable_chips.push_default();
        let mut bt_chip = Chip_Bluetooth::new();
        let chip_state = match state {
            "up" => State::ON,
            _ => State::OFF,
        };
        if bt_type == "ble" {
            bt_chip.set_low_energy(Chip_Radio { state: chip_state, ..Default::default() });
        } else {
            bt_chip.set_classic(Chip_Radio { state: chip_state, ..Default::default() });
        }
        mutable_chips[0].set_bt(bt_chip);
        result.write_to_bytes().unwrap()
    }

    #[test]
    fn test_radio_ble() {
        test_command(
            "netsim-cli radio ble down 1000",
            GrpcMethod::UpdateDevice,
            get_expected_radio("1000", "ble", "down"),
        );
        test_command(
            "netsim-cli radio ble up 1000",
            GrpcMethod::UpdateDevice,
            get_expected_radio("1000", "ble", "up"),
        );
    }

    #[test]
    fn test_radio_classic() {
        test_command(
            "netsim-cli radio classic down 100",
            GrpcMethod::UpdateDevice,
            get_expected_radio("100", "classic", "down"),
        );
        test_command(
            "netsim-cli radio classic up 100",
            GrpcMethod::UpdateDevice,
            get_expected_radio("100", "classic", "up"),
        );
    }

    fn get_expected_move(device_serial: &str, x: f32, y: f32, z: Option<f32>) -> Vec<u8> {
        let mut result = frontend::UpdateDeviceRequest::new();
        let mutable_device = result.mut_device();
        mutable_device.set_device_serial(device_serial.to_owned());
        mutable_device.set_position(Position {
            x: x,
            y: y,
            z: z.unwrap_or_default(),
            ..Default::default()
        });
        result.write_to_bytes().unwrap()
    }

    #[test]
    fn test_move_int() {
        test_command(
            "netsim-cli move 1 1 2 3",
            GrpcMethod::UpdateDevice,
            get_expected_move("1", 1.0, 2.0, Some(3.0)),
        )
    }

    #[test]
    fn test_move_float() {
        test_command(
            "netsim-cli move 1000 1.2 3.4 5.6",
            GrpcMethod::UpdateDevice,
            get_expected_move("1000", 1.2, 3.4, Some(5.6)),
        )
    }

    #[test]
    fn test_move_mixed() {
        test_command(
            "netsim-cli move 1000 1.1 2 3.4",
            GrpcMethod::UpdateDevice,
            get_expected_move("1000", 1.1, 2.0, Some(3.4)),
        )
    }

    #[test]
    fn test_move_no_z() {
        test_command(
            "netsim-cli move 1000 1.2 3.4",
            GrpcMethod::UpdateDevice,
            get_expected_move("1000", 1.2, 3.4, None),
        )
    }

    #[test]
    fn test_devices() {
        test_command("netsim-cli devices", GrpcMethod::GetDevices, Vec::new())
    }

    fn get_expected_capture(device_serial: &str, set_capture: bool) -> Vec<u8> {
        let mut result = frontend::SetPacketCaptureRequest::new();
        result.set_device_serial(device_serial.to_owned());
        result.set_capture(set_capture);
        result.write_to_bytes().unwrap()
    }

    #[test]
    fn test_capture_mixed_case() {
        test_command(
            "netsim-cli capture True 10",
            GrpcMethod::SetPacketCapture,
            get_expected_capture("10", true),
        );
        test_command(
            "netsim-cli capture False 1000",
            GrpcMethod::SetPacketCapture,
            get_expected_capture("1000", false),
        )
    }

    #[test]
    fn test_capture_uppercase() {
        test_command(
            "netsim-cli capture TRUE 1000",
            GrpcMethod::SetPacketCapture,
            get_expected_capture("1000", true),
        );
        test_command(
            "netsim-cli capture FALSE 1000",
            GrpcMethod::SetPacketCapture,
            get_expected_capture("1000", false),
        )
    }

    #[test]
    fn test_capture_lowercase() {
        test_command(
            "netsim-cli capture true 1000",
            GrpcMethod::SetPacketCapture,
            get_expected_capture("1000", true),
        );
        test_command(
            "netsim-cli capture false 1000",
            GrpcMethod::SetPacketCapture,
            get_expected_capture("1000", false),
        )
    }

    #[test]
    fn test_reset() {
        test_command("netsim-cli reset", GrpcMethod::Reset, Vec::new())
    }
}
