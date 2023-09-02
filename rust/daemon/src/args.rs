// Copyright 2023 Google LLC
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

use clap::Parser;

#[derive(Debug, Parser)]
pub struct NetsimdArgs {
    /// File descriptor start up info proto
    #[arg(short = 's', long)]
    pub fd_startup_str: Option<String>,

    /// Disable grpc server for CLI
    #[arg(long)]
    pub no_cli_ui: bool,

    /// Disable web server
    #[arg(long)]
    pub no_web_ui: bool,

    /// Enable packet capture
    #[arg(long)]
    pub pcap: bool,

    /// Disable Address Reuse for test model
    #[arg(long)]
    pub disable_address_reuse: bool,

    /// Set custom hci port
    #[arg(long, alias = "hci_port")]
    pub hci_port: Option<u32>,

    /// Netsimd primary instance to connect to
    #[arg(short, long, visible_alias = "rootcanal_instance_num")]
    pub primary_instance: Option<u16>,

    /// Netsimd instance number
    #[arg(short, long, visible_alias = "instance_num")]
    pub instance: Option<u16>,

    /// Set whether log messages go to stderr instead of logfiles
    #[arg(short, long)]
    pub logtostderr: bool,

    /// Enable development mode. This will include additional features
    #[arg(short, long)]
    pub dev: bool,

    /// Set the vsock port number to be listened by the frontend grpc server
    #[arg(short, long)]
    pub vsock: Option<u16>,
}
