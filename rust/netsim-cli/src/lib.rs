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
mod response;

use args::NetsimArgs;
use clap::Parser;
use frontend_client_cxx::{ffi, send_grpc, GrpcMethod};

/// helper function to send the Grpc request and handle the response
fn perform_request(
    command: args::Command,
    client: cxx::UniquePtr<ffi::FrontendClient>,
    grpc_method: GrpcMethod,
    request: Vec<u8>,
) -> Result<(), String> {
    let continuous = match command {
        args::Command::Devices(ref cmd) => cmd.continuous,
        _ => false,
    };
    loop {
        let client_result = send_grpc(&client, &grpc_method, &request);
        if client_result.is_ok() {
            command.print_response(client_result.byte_vec().as_slice());
            if !continuous {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        } else {
            return Err(format!("Grpc call error: {}", client_result.err()));
        }
    }
}

#[no_mangle]
/// main function for netsim CLI to be called by C wrapper netsim-cl
pub extern "C" fn rust_main() {
    let args = NetsimArgs::parse();
    if matches!(args.command, args::Command::Gui) {
        browser::open("http://localhost:7681/");
        return;
    }
    let grpc_method = args.command.grpc_method();
    let request = args.command.get_request_bytes();
    let client = ffi::new_frontend_client();
    if client.is_null() {
        eprintln!("Unable to create frontend client. Please ensure netsimd is running.");
        return;
    }
    if let Err(e) = perform_request(args.command, client, grpc_method, request) {
        eprintln!("{e}");
    }
}
