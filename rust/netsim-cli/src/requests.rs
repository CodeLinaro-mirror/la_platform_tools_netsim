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

extern crate frontend_proto;

use crate::args::{self, Command};

#[derive(Debug, PartialEq, Eq)]
pub enum GrpcMethod {
    GetVersion,
    UpdateDevice,
    GetDevices,
    SetPacketCapture,
    Reset,
}

impl args::Command {
    pub fn grpc_method(&self) -> GrpcMethod {
        match self {
            Command::Version => GrpcMethod::GetVersion,
            Command::Radio(_) => GrpcMethod::UpdateDevice,
            Command::Move(_) => GrpcMethod::UpdateDevice,
            Command::Devices => GrpcMethod::GetDevices,
            Command::Capture(_) => GrpcMethod::SetPacketCapture,
            Command::Reset => GrpcMethod::Reset,
            Command::Ui => {
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
    use serde_json::{json, Value};

    fn test_command(
        command: &str,
        expected_grpc_method: GrpcMethod,
        expected_json: serde_json::Value,
    ) {
        let command = NetsimArgs::parse_from(command.split_whitespace()).command;
        assert_eq!(expected_grpc_method, command.grpc_method());
        assert_eq!(
            serde_json::from_str::<Value>(&command.request_json()).unwrap_or(json!(null)),
            expected_json
        );
    }

    #[test]
    fn test_version_request() {
        test_command("netsim-cli version", GrpcMethod::GetVersion, json!({}))
    }

    //TODO: Add more radio tests once bt/hci are added
    #[test]
    fn test_radio_dummy() {
        test_command(
            "netsim-cli radio ble down 1000",
            GrpcMethod::UpdateDevice,
            json!({ "device": null }),
        )
    }

    fn expected_move_json(x: f32, y: f32, z: Option<f32>) -> Value {
        json!({
            "device": {
                "device_serial": "1000",
                "name": "",
                "visible": false,
                "position": {
                    "x": (x as f64 * 100.0).trunc() / 100.0,  // workaround for serde_json's widening to f64 for testing
                    "y": (y as f64 * 100.0).trunc() / 100.0,
                    "z": (z.unwrap_or_default() as f64 * 100.0).trunc() / 100.0,
                },
                "orientation": null,
                "chips": [],
            }
        })
    }

    #[test]
    fn test_move_int() {
        test_command(
            "netsim-cli move 1000 1 2 3",
            GrpcMethod::UpdateDevice,
            expected_move_json(1.0, 2.0, Some(3.0)),
        )
    }

    #[test]
    fn test_move_float() {
        test_command(
            "netsim-cli move 1000 1.1 1.1 1.1",
            GrpcMethod::UpdateDevice,
            expected_move_json(1.1, 1.1, Some(1.1)),
        )
    }

    #[test]
    fn test_move_mixed() {
        test_command(
            "netsim-cli move 1000 1.1 2 3.4",
            GrpcMethod::UpdateDevice,
            expected_move_json(1.1, 2.0, Some(3.4)),
        )
    }

    #[test]
    fn test_move_no_z() {
        test_command(
            "netsim-cli move 1000 1.2 3.4",
            GrpcMethod::UpdateDevice,
            expected_move_json(1.2, 3.4, None),
        )
    }

    #[test]
    fn test_devices() {
        test_command("netsim-cli devices", GrpcMethod::GetDevices, json!({}))
    }

    #[test]
    fn test_capture_mixed_case() {
        test_command(
            "netsim-cli capture True 1000",
            GrpcMethod::SetPacketCapture,
            json!({
                "capture": true,
                "device_serial": "1000"
            }),
        );
        test_command(
            "netsim-cli capture False 1000",
            GrpcMethod::SetPacketCapture,
            json!({
                "capture": false,
                "device_serial": "1000"
            }),
        )
    }

    #[test]
    fn test_capture_uppercase() {
        test_command(
            "netsim-cli capture TRUE 1000",
            GrpcMethod::SetPacketCapture,
            json!({
                "capture": true,
                "device_serial": "1000"
            }),
        );
        test_command(
            "netsim-cli capture FALSE 1000",
            GrpcMethod::SetPacketCapture,
            json!({
                "capture": false,
                "device_serial": "1000"
            }),
        )
    }

    #[test]
    fn test_capture_lowercase() {
        test_command(
            "netsim-cli capture true 1000",
            GrpcMethod::SetPacketCapture,
            json!({
                "capture": true,
                "device_serial": "1000"
            }),
        );
        test_command(
            "netsim-cli capture false 1000",
            GrpcMethod::SetPacketCapture,
            json!({
                "capture": false,
                "device_serial": "1000"
            }),
        )
    }

    #[test]
    fn test_reset() {
        test_command("netsim-cli reset", GrpcMethod::Reset, json!({}))
    }
}
