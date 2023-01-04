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

//! Command Line Interface for Netsim

mod args;
mod browser;
mod requests;
use args::NetsimArgs;
use clap::Parser;

fn create_request(args: NetsimArgs) -> args::CommandType {
    // Request format not available yet. Function will return the command as is.
    args.command
}

fn call_command(command: &args::CommandType, mut writer: impl std::io::Write) {
    // Commands are not implemented yet.
    // Each of the statements will be replaced with function calls once implemented.
    let err_msg = "Error writing output";
    match command {
        args::CommandType::Version => {
            requests::get_version(writer);
        }
        args::CommandType::Radio(cmd) => {
            writeln!(
                writer,
                "(Not yet implemented.) Radio Command bt_type is: {}, status is: {}, serial is: {}",
                cmd.bt_type, cmd.status, cmd.device_serial
            )
            .expect(err_msg);
        }
        args::CommandType::Move(cmd) => {
            writeln!(
                writer,
                "(Not yet implemented.) Move Command serial is: {}, x is: {}, y is: {}, z is: {}",
                cmd.device_serial, cmd.x, cmd.y, cmd.z
            )
            .expect(err_msg);
        }
        args::CommandType::Devices => {
            writeln!(writer, "(Not yet implemented.) Display devices: ").expect(err_msg);
        }
        args::CommandType::Capture(cmd) => {
            writeln!(
                writer,
                "(Not yet implemented.) Capture Command state is: {}, serial is: {}",
                cmd.state, cmd.device_serial
            )
            .expect(err_msg);
        }
        args::CommandType::Reset => {
            writeln!(writer, "(Not yet implemented.) Reset netsim: ").expect(err_msg);
        }
        args::CommandType::Ui => {
            browser::open("https://google.com");
        }
    }
}

fn main() {
    let args = NetsimArgs::parse();
    let command = create_request(args);
    call_command(&command, &mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to parse args for create_request tests
    fn get_request_from_arg(test_args: &str) -> args::CommandType {
        create_request(NetsimArgs::parse_from(test_args.split_whitespace()))
    }

    #[test]
    fn version_request() {
        if let args::CommandType::Version = get_request_from_arg("netsim-cli version") {
            assert!(true)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn radio_request() {
        if let args::CommandType::Radio(cmd) = get_request_from_arg("netsim-cli radio ble up 1000")
        {
            assert!(
                cmd.bt_type == args::BtType::Ble
                    && cmd.status == args::UpDownStatus::Up
                    && cmd.device_serial == "1000"
            )
        } else {
            assert!(false)
        }
    }

    #[test]
    fn move_request() {
        if let args::CommandType::Move(cmd) = get_request_from_arg("netsim-cli move 1000 2 3 4") {
            assert!(cmd.device_serial == "1000" && cmd.x == 2.0 && cmd.y == 3.0 && cmd.z == 4.0)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn devices_request() {
        if let args::CommandType::Devices = get_request_from_arg("netsim-cli devices") {
            assert!(true)
        } else {
            assert!(false)
        }
    }

    #[test]
    fn capture_request() {
        if let args::CommandType::Capture(cmd) =
            get_request_from_arg("netsim-cli capture true 1000")
        {
            assert!(cmd.state == args::BoolState::True && cmd.device_serial == "1000")
        } else {
            assert!(false)
        }
    }

    #[test]
    fn reset_request() {
        if let args::CommandType::Reset = get_request_from_arg("netsim-cli reset") {
            assert!(true)
        } else {
            assert!(false)
        }
    }
    // TODO: add tests for call_command when commands are implemented. (Will need some scaffold for server)
}
