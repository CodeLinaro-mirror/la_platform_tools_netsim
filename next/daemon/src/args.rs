// Copyright 2023-2025 The Android Open Source Project

use clap::Parser;
use std::env;

#[derive(Debug, Parser, Default)]
pub struct Args {
    /// File descriptor start up info proto
    #[arg(short = 's', long, alias = "fd_startup_str")]
    pub fd_startup_str: Option<String>,

    /// Enables connector mode to forward packets to another instance.
    #[arg(short, long, alias = "connector_instance", visible_alias = "connector_instance_num")]
    pub connector_instance: Option<u16>,

    /// Netsimd instance number
    #[arg(short, long, visible_alias = "instance_num")]
    pub instance: Option<u16>,

    /// Set whether log messages go to stderr instead of logfiles
    #[arg(short, long)]
    pub logtostderr: bool,

    /// Enable development mode. This will include additional features
    #[arg(short, long)]
    pub dev: bool,

    /// Disable netsimd from shutting down automatically.
    /// WARNING: This flag is for development purpose. netsimd will not shutdown without SIGKILL.
    #[arg(long, alias = "no_shutdown")]
    pub no_shutdown: bool,

    /// Enable packet capture
    #[arg(long)]
    pub pcap: bool,

    /// Entering Verbose mode
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Print Netsimd version information
    #[arg(long)]
    pub version: bool,

    /// gRPC port for the netsim service
    #[arg(long, alias = "grpc_port")]
    pub grpc_port: Option<u16>,
}

impl Args {
    // Return the command line args along with any NETSIM_ARGS from
    // the environment.
    pub(crate) fn parse() -> Args {
        let mut args: Vec<String> = env::args().collect();
        // This simple split WILL FAIL on quoted values like: --host "my custom host"
        if let Ok(env_str) = env::var("NETSIM_ARGS") {
            args.extend(env_str.split_whitespace().map(|s| s.to_string()));
        }
        Args::parse_from(args)
    }
}
