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

fn main() {
    let args = NetsimArgs::parse();
    if matches!(args.command, args::Command::Ui) {
        browser::open("https://google.com"); //TODO: update to open netsim ui directly
        return;
    }
    let _grpc_method = args.command.grpc_method();
    let _json_string = args.command.request_json();
    //TODO: update to use grpc_method and _json_string with SendGrpc function from frontend-netsim-cxx
}
