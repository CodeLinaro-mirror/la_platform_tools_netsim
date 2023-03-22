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

use args::{BinaryProtobuf, NetsimArgs};
use clap::Parser;
use cxx::UniquePtr;
use frontend_client_cxx::ffi::{new_frontend_client, ClientResult, FrontendClient, GrpcMethod};
use frontend_client_cxx::{ClientResponseReadable, ClientResponseReader};

struct PcapHandler {
    file: String,
}

impl Drop for PcapHandler {
    // handle cleanup for file
    fn drop(&mut self) {
        // TODO: close the open file and remove print statement
        println!("Dropping reader file: {}", self.file);
    }
}

impl ClientResponseReadable for PcapHandler {
    // function to handle writing each chunk to file
    fn handle_chunk(&self, chunk: &[u8]) {
        //TODO: write chunk to file
        println!("handling chunk of length {} on file {}", chunk.len(), self.file);
    }
    // function to handle error response
    fn handle_error(&self, error_code: u32, error_message: &str) {
        println!(
            "Handling error code: {}, msg: {}, on file: {}",
            error_code, error_message, self.file
        );
    }
}

// helper function to process streaming Grpc request
fn perform_streaming_request(client: &cxx::UniquePtr<FrontendClient>) -> UniquePtr<ClientResult> {
    client.get_pcap(
        &Vec::new(),
        &ClientResponseReader {
            handler: Box::new(PcapHandler { file: "placeholder file name".to_string() }),
        },
    )
}

/// helper function to send the Grpc request and handle the response
fn perform_request(
    command: args::Command,
    client: cxx::UniquePtr<FrontendClient>,
    grpc_method: GrpcMethod,
    requests: Vec<BinaryProtobuf>,
    verbose: bool,
) -> Result<(), String> {
    for req in requests {
        let result = match command {
            // Continuous option sends the gRPC call every second
            args::Command::Devices(ref cmd) if cmd.continuous => loop {
                process_result(&command, client.send_grpc(&grpc_method, &req), verbose)?;
                std::thread::sleep(std::time::Duration::from_secs(1));
            },
            // Get Pcap use streaming gRPC reader request
            args::Command::Pcap(args::Pcap::Get(_)) => perform_streaming_request(&client),
            // All other commands use a single gRPC call
            _ => client.send_grpc(&grpc_method, &req),
        };
        process_result(&command, result, verbose)?;
    }
    Ok(())
}

/// Check and handle the gRPC call result
fn process_result(
    command: &args::Command,
    result: UniquePtr<ClientResult>,
    verbose: bool,
) -> Result<(), String> {
    if result.is_ok() {
        command.print_response(result.byte_vec().as_slice(), verbose);
    } else {
        return Err(format!("Grpc call error: {}", result.err()));
    }
    Ok(())
}
#[no_mangle]
/// main Rust netsim CLI function to be called by C wrapper netsim.cc
pub extern "C" fn rust_main() {
    let args = NetsimArgs::parse();
    if matches!(args.command, args::Command::Gui) {
        browser::open("http://localhost:7681/");
        return;
    }
    // TODO: remove warning once pcap commands are implemented
    if matches!(args.command, args::Command::Pcap(args::Pcap::Get(_))) {
        eprintln!("Warning: GetPcap is not fully implemented yet!");
    }
    let grpc_method = args.command.grpc_method();
    let client = new_frontend_client();
    if client.is_null() {
        eprintln!("Unable to create frontend client. Please ensure netsimd is running.");
        return;
    }
    // Handle where there are potentially multiple requests
    let reqs = if matches!(args.command, args::Command::Pcap(args::Pcap::Patch(_))) {
        args.command.get_requests(&client)
    } else {
        vec![args.command.get_request_bytes()]
    };
    if let Err(e) = perform_request(args.command, client, grpc_method, reqs, args.verbose) {
        eprintln!("{e}");
    }
}
